use lora_phy::{
    mod_params::PacketStatus,
    mod_traits::{DesiredIrqState, IrqState},
};

pub use crate::radio::{Bandwidth, CodingRate, RfConfig, RxQuality, SpreadingFactor, TxConfig};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Error<E>(pub E);

impl<R> From<Error<R>> for super::Error<R> {
    fn from(radio_error: Error<R>) -> super::Error<R> {
        super::Error::Radio(radio_error.0)
    }
}

/// An asynchronous timer that allows the state machine to await
/// between RX windows.
pub trait Timer {
    fn reset(&mut self);

    /// Wait until millis milliseconds after reset has passed
    async fn at(&mut self, millis: u64);

    /// Delay for millis milliseconds
    async fn delay_ms(&mut self, millis: u64);
}

/// An asynchronous radio implementation that can transmit and receive data.
pub trait PhyRxTx: Sized {
    #[cfg(feature = "defmt")]
    type PhyError: defmt::Format;

    #[cfg(not(feature = "defmt"))]
    type PhyError;

    /// Transmit data buffer with the given transceiver configuration. The returned future
    /// should only complete once data have been transmitted.
    async fn tx(&mut self, config: TxConfig, buf: &[u8]) -> Result<u32, Self::PhyError>;

    /// Configures the radio to receive data. This future should not actually await the data itself.
    async fn setup_rx(&mut self, config: RfConfig) -> Result<(), Self::PhyError>;

    /// Receive data into the provided buffer with the given transceiver configuration. The returned
    /// future should only complete when RX data have been received. Furthermore, it should be
    /// possible to await the future again without settings up the receive config again.
    async fn rx(&mut self, rx_buf: &mut [u8]) -> Result<(usize, RxQuality), Self::PhyError> {
        loop {
            if let IrqState::Done(length, lq) =
                self.rx_until_state(rx_buf, DesiredIrqState::Done).await?
            {
                return Ok((length as usize, RxQuality::new(lq.rssi, lq.snr as i8)));
            }
        }
    }

    /// Receive data into the provided buffer with the given transceiver configuration. The returned
    /// future should only complete when the target RX state has been reached. Furthermore, it should be
    /// possible to await the future again without settings up the receive config again.
    async fn rx_until_state(
        &mut self,
        rx_buf: &mut [u8],
        _desired_state: DesiredIrqState,
    ) -> Result<IrqState, Self::PhyError> {
        let (length, lq) = self.rx(rx_buf).await?;
        Ok(IrqState::Done(length as u8, PacketStatus { rssi: lq.rssi(), snr: lq.snr() as i16 }))
    }

    /// Puts the radio into a low-power mode
    async fn low_power(&mut self) -> Result<(), Self::PhyError> {
        Ok(())
    }
}
