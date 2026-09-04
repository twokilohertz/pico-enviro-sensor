use embassy_sync::{blocking_mutex::raw::NoopRawMutex, pubsub};
use scd4x::types::SensorData;

/// Message on the [MessageBus]
#[derive(Clone, Copy)]
pub(crate) enum Message {
    /// System has started up
    Startup,
    /// New sensor data was read from the sensor
    SensorMeasurement(SensorData),
    /// Running tasks should quit
    #[allow(unused)]
    Shutdown,
}

/// Maximum number of backlogged [Message]s on the [MessageBus]
pub(crate) const MAX_N_MESSAGES: usize = 4;
/// Maximum number of subscribers on the [MessageBus] (display output, Wi-Fi & bluetooth)
pub(crate) const MAX_N_SUBSCRIBERS: usize = 3;
/// Maximum number of publishers on the [MessageBus] (sensor read & input, +1 for entrypoint announcing system startup)
pub(crate) const MAX_N_PUBLISHERS: usize = 3;

/// Type alias for the system-wide message bus
pub(crate) type MessageBus = pubsub::PubSubChannel<
    NoopRawMutex,
    Message,
    MAX_N_MESSAGES,
    MAX_N_SUBSCRIBERS,
    MAX_N_PUBLISHERS,
>;
/// Type alias for a system-wide message bus subscriber
pub(crate) type MessageBusSubscriber<'a> = pubsub::Subscriber<
    'a,
    NoopRawMutex,
    Message,
    MAX_N_MESSAGES,
    MAX_N_SUBSCRIBERS,
    MAX_N_PUBLISHERS,
>;
/// Type alias for a system-wide message bus publisher
pub(crate) type MessageBusPublisher<'a> = pubsub::Publisher<
    'a,
    NoopRawMutex,
    Message,
    MAX_N_MESSAGES,
    MAX_N_SUBSCRIBERS,
    MAX_N_PUBLISHERS,
>;
