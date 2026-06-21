use napi_derive::napi;

/// What a node does when a message reaches it with a full transmission queue.
#[napi(string_enum)]
pub enum CongestionControl {
  /// Drop the message (the default for `put`/`delete`).
  Drop,
  /// Block the caller until the queue has room.
  Block,
  /// When transmitting a message in a node with a full queue, the node will wait for queue to progress,
  /// but only for the first message sent with this strategy; other messages will be dropped.
  BlockFirst,
}

impl From<CongestionControl> for zenoh::qos::CongestionControl {
  fn from(value: CongestionControl) -> Self {
    match value {
      CongestionControl::Drop => zenoh::qos::CongestionControl::Drop,
      CongestionControl::Block => zenoh::qos::CongestionControl::Block,
      CongestionControl::BlockFirst => zenoh::qos::CongestionControl::BlockFirst,
    }
  }
}

/// Priority of a publication. Listed highest to lowest; `Data` is the default.
#[napi(string_enum)]
pub enum Priority {
  RealTime,
  InteractiveHigh,
  InteractiveLow,
  DataHigh,
  Data,
  DataLow,
  Background,
}

impl From<Priority> for zenoh::qos::Priority {
  fn from(value: Priority) -> Self {
    use zenoh::qos::Priority::*;
    match value {
      Priority::RealTime => RealTime,
      Priority::InteractiveHigh => InteractiveHigh,
      Priority::InteractiveLow => InteractiveLow,
      Priority::DataHigh => DataHigh,
      Priority::Data => Data,
      Priority::DataLow => DataLow,
      Priority::Background => Background,
    }
  }
}

/// Reliability of message delivery.
///
/// **NOTE**: as in Zenoh itself, reliability does not currently trigger wire
/// retransmission; it is a marker that may influence link selection.
#[napi(string_enum)]
pub enum Reliability {
  /// Messages may be lost.
  BestEffort,
  /// Messages are guaranteed to be delivered (the default).
  Reliable,
}

impl From<Reliability> for zenoh::qos::Reliability {
  fn from(value: Reliability) -> Self {
    match value {
      Reliability::BestEffort => zenoh::qos::Reliability::BestEffort,
      Reliability::Reliable => zenoh::qos::Reliability::Reliable,
    }
  }
}

// Reverse conversions, used to read QoS back off declared entities.

impl From<zenoh::qos::CongestionControl> for CongestionControl {
  fn from(value: zenoh::qos::CongestionControl) -> Self {
    match value {
      zenoh::qos::CongestionControl::Drop => CongestionControl::Drop,
      zenoh::qos::CongestionControl::Block => CongestionControl::Block,
      zenoh::qos::CongestionControl::BlockFirst => CongestionControl::BlockFirst,
    }
  }
}

impl From<zenoh::qos::Priority> for Priority {
  fn from(value: zenoh::qos::Priority) -> Self {
    use zenoh::qos::Priority::*;
    match value {
      RealTime => Priority::RealTime,
      InteractiveHigh => Priority::InteractiveHigh,
      InteractiveLow => Priority::InteractiveLow,
      DataHigh => Priority::DataHigh,
      Data => Priority::Data,
      DataLow => Priority::DataLow,
      Background => Priority::Background,
    }
  }
}

impl From<zenoh::qos::Reliability> for Reliability {
  fn from(value: zenoh::qos::Reliability) -> Self {
    match value {
      zenoh::qos::Reliability::BestEffort => Reliability::BestEffort,
      zenoh::qos::Reliability::Reliable => Reliability::Reliable,
    }
  }
}
