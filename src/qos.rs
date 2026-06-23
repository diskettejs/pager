use napi_derive::napi;
use zenoh::qos::{
  CongestionControl as ZCongestionControl, Priority as ZPriority, Reliability as ZReliability,
};
use zenoh::sample::Locality as ZLocality;

/// The congestion control to apply when routing data.
#[napi(string_enum)]
pub enum CongestionControl {
  /// Drop the message when the queue is full.
  Drop,
  /// Wait for the queue to progress when it is full.
  Block,
  /// Block only the first message sent with this strategy; drop the rest.
  BlockFirst,
}

impl From<CongestionControl> for ZCongestionControl {
  fn from(value: CongestionControl) -> Self {
    match value {
      CongestionControl::Drop => Self::Drop,
      CongestionControl::Block => Self::Block,
      CongestionControl::BlockFirst => Self::BlockFirst,
    }
  }
}

impl From<ZCongestionControl> for CongestionControl {
  fn from(value: ZCongestionControl) -> Self {
    match value {
      ZCongestionControl::Drop => Self::Drop,
      ZCongestionControl::Block => Self::Block,
      ZCongestionControl::BlockFirst => Self::BlockFirst,
    }
  }
}

/// The priority of a message when routing.
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

impl From<Priority> for ZPriority {
  fn from(value: Priority) -> Self {
    match value {
      Priority::RealTime => Self::RealTime,
      Priority::InteractiveHigh => Self::InteractiveHigh,
      Priority::InteractiveLow => Self::InteractiveLow,
      Priority::DataHigh => Self::DataHigh,
      Priority::Data => Self::Data,
      Priority::DataLow => Self::DataLow,
      Priority::Background => Self::Background,
    }
  }
}

impl From<ZPriority> for Priority {
  fn from(value: ZPriority) -> Self {
    match value {
      ZPriority::RealTime => Self::RealTime,
      ZPriority::InteractiveHigh => Self::InteractiveHigh,
      ZPriority::InteractiveLow => Self::InteractiveLow,
      ZPriority::DataHigh => Self::DataHigh,
      ZPriority::Data => Self::Data,
      ZPriority::DataLow => Self::DataLow,
      ZPriority::Background => Self::Background,
    }
  }
}

/// The reliability to apply when routing data.
#[napi(string_enum)]
pub enum Reliability {
  BestEffort,
  Reliable,
}

impl From<Reliability> for ZReliability {
  fn from(value: Reliability) -> Self {
    match value {
      Reliability::BestEffort => Self::BestEffort,
      Reliability::Reliable => Self::Reliable,
    }
  }
}

impl From<ZReliability> for Reliability {
  fn from(value: ZReliability) -> Self {
    match value {
      ZReliability::BestEffort => Self::BestEffort,
      ZReliability::Reliable => Self::Reliable,
    }
  }
}

/// Restricts which entities (relative to this session) data is routed to/from.
#[napi(string_enum)]
pub enum Locality {
  /// Only entities in the same session.
  SessionLocal,
  /// Only remote entities (not in the same session).
  Remote,
  /// Both local and remote entities.
  Any,
}

impl From<Locality> for ZLocality {
  fn from(value: Locality) -> Self {
    match value {
      Locality::SessionLocal => Self::SessionLocal,
      Locality::Remote => Self::Remote,
      Locality::Any => Self::Any,
    }
  }
}

impl From<ZLocality> for Locality {
  fn from(value: ZLocality) -> Self {
    match value {
      ZLocality::SessionLocal => Self::SessionLocal,
      ZLocality::Remote => Self::Remote,
      ZLocality::Any => Self::Any,
    }
  }
}
