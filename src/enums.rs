//! QoS / addressing enums. NAPI `string_enum` → values are the Rust variant names
//! verbatim (PascalCase). `From<zenoh::…>` maps inbound (samples), `From<…>` the
//! other way maps outbound (option values set on builders).

use napi_derive::napi;

#[napi(string_enum)]
pub enum SampleKind {
  Put,
  Delete,
}

impl From<zenoh::sample::SampleKind> for SampleKind {
  fn from(k: zenoh::sample::SampleKind) -> Self {
    match k {
      zenoh::sample::SampleKind::Put => SampleKind::Put,
      zenoh::sample::SampleKind::Delete => SampleKind::Delete,
    }
  }
}

#[napi(string_enum)]
pub enum CongestionControl {
  Drop,
  Block,
}

impl From<zenoh::qos::CongestionControl> for CongestionControl {
  fn from(c: zenoh::qos::CongestionControl) -> Self {
    match c {
      zenoh::qos::CongestionControl::Drop => CongestionControl::Drop,
      zenoh::qos::CongestionControl::Block => CongestionControl::Block,
    }
  }
}

impl From<CongestionControl> for zenoh::qos::CongestionControl {
  fn from(c: CongestionControl) -> Self {
    match c {
      CongestionControl::Drop => zenoh::qos::CongestionControl::Drop,
      CongestionControl::Block => zenoh::qos::CongestionControl::Block,
    }
  }
}

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

impl From<zenoh::qos::Priority> for Priority {
  fn from(p: zenoh::qos::Priority) -> Self {
    use zenoh::qos::Priority as Z;
    match p {
      Z::RealTime => Priority::RealTime,
      Z::InteractiveHigh => Priority::InteractiveHigh,
      Z::InteractiveLow => Priority::InteractiveLow,
      Z::DataHigh => Priority::DataHigh,
      Z::Data => Priority::Data,
      Z::DataLow => Priority::DataLow,
      Z::Background => Priority::Background,
    }
  }
}

impl From<Priority> for zenoh::qos::Priority {
  fn from(p: Priority) -> Self {
    use zenoh::qos::Priority as Z;
    match p {
      Priority::RealTime => Z::RealTime,
      Priority::InteractiveHigh => Z::InteractiveHigh,
      Priority::InteractiveLow => Z::InteractiveLow,
      Priority::DataHigh => Z::DataHigh,
      Priority::Data => Z::Data,
      Priority::DataLow => Z::DataLow,
      Priority::Background => Z::Background,
    }
  }
}

#[napi(string_enum)]
pub enum Locality {
  SessionLocal,
  Remote,
  Any,
}

impl From<Locality> for zenoh::sample::Locality {
  fn from(l: Locality) -> Self {
    match l {
      Locality::SessionLocal => zenoh::sample::Locality::SessionLocal,
      Locality::Remote => zenoh::sample::Locality::Remote,
      Locality::Any => zenoh::sample::Locality::Any,
    }
  }
}
