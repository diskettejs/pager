use napi::bindgen_prelude::{
  BigInt, BigInt64Array, BigUint64Array, Float32Array, Float64Array, Int8Array, Int16Array,
  Int32Array, Uint8Array, Uint16Array, Uint32Array,
};
use napi_derive::napi;
use zenoh::bytes::ZBytes;
use zenoh_ext::{VarInt, ZDeserializer as ZDe, ZSerializer as ZSer};

use crate::bytes::Bytes;

/// Map a `zenoh-ext` deserialization failure into a JS error.
fn de_err(_: zenoh_ext::ZDeserializeError) -> napi::Error {
  napi::Error::from_reason("deserialization error")
}

/// Extract an `i128` from a JS `BigInt`, validating range directly from the
/// little-endian magnitude words.
///
/// We do not use napi's `BigInt::get_i128`: its lossless flag is inverted in the
/// `i128::MIN` branch (`len > 2` where it should be `len == 2`), so it reports
/// the exact, in-range `i128::MIN` as lossy while reporting a *truncated*
/// out-of-range value as lossless. Reading `words`/`sign_bit` ourselves is exact.
fn i128_from_bigint(value: &BigInt) -> napi::Result<i128> {
  let out_of_range = || napi::Error::from_reason("BigInt out of i128 range");
  // `words` is the magnitude, little-endian in 64-bit limbs. Anything above the
  // low two limbs means the magnitude is >= 2^128.
  if value.words.iter().skip(2).any(|&w| w != 0) {
    return Err(out_of_range());
  }
  let lo = value.words.first().copied().unwrap_or(0) as u128;
  let hi = value.words.get(1).copied().unwrap_or(0) as u128;
  let mag = lo | (hi << 64);
  if value.sign_bit {
    // Negative: magnitude must be <= 2^127 (== i128::MIN.unsigned_abs()).
    // `wrapping_neg` maps mag == 2^127 (which casts to i128::MIN) to i128::MIN.
    if mag > i128::MIN.unsigned_abs() {
      return Err(out_of_range());
    }
    Ok((mag as i128).wrapping_neg())
  } else {
    // Non-negative: magnitude must be <= 2^127 - 1 (== i128::MAX).
    if mag > i128::MAX as u128 {
      return Err(out_of_range());
    }
    Ok(mag as i128)
  }
}

/// Streaming serializer implementing the Zenoh serialization format.
///
/// Serializing values one after another is equivalent to serializing a tuple of
/// those values. Call [`ZSerializer::finish`] to consume the serializer and
/// produce the resulting [`Bytes`]; the serializer cannot be used afterwards.
#[napi]
pub struct Serializer {
  // `Option` so `finish` can consume the inner serializer (`ZSer::finish(self)`)
  // under napi's `&mut self` methods. `take`-ing leaves `None`, after which every
  // method throws.
  inner: Option<ZSer>,
}

impl Serializer {
  fn writer(&mut self) -> napi::Result<&mut ZSer> {
    self
      .inner
      .as_mut()
      .ok_or_else(|| napi::Error::from_reason("serializer already finished"))
  }
}

#[napi]
impl Serializer {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Some(ZSer::new()),
    }
  }

  // --- integer scalars representable as a JS `number` -----------------------

  #[napi]
  pub fn i8(&mut self, value: i8) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  #[napi]
  pub fn i16(&mut self, value: i16) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  #[napi]
  pub fn i32(&mut self, value: i32) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  #[napi]
  pub fn u8(&mut self, value: u8) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  #[napi]
  pub fn u16(&mut self, value: u16) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  #[napi]
  pub fn u32(&mut self, value: u32) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  // --- 64/128-bit integer scalars carried as `BigInt` -----------------------

  #[napi]
  pub fn i64(&mut self, value: BigInt) -> napi::Result<()> {
    let (val, lossless) = value.get_i64();
    if !lossless {
      return Err(napi::Error::from_reason("BigInt out of i64 range"));
    }
    self.writer()?.serialize(val);
    Ok(())
  }

  #[napi]
  pub fn u64(&mut self, value: BigInt) -> napi::Result<()> {
    let (signed, val, lossless) = value.get_u64();
    if signed || !lossless {
      return Err(napi::Error::from_reason("BigInt out of u64 range"));
    }
    self.writer()?.serialize(val);
    Ok(())
  }

  #[napi]
  pub fn i128(&mut self, value: BigInt) -> napi::Result<()> {
    let val = i128_from_bigint(&value)?;
    self.writer()?.serialize(val);
    Ok(())
  }

  #[napi]
  pub fn u128(&mut self, value: BigInt) -> napi::Result<()> {
    let (signed, val, lossless) = value.get_u128();
    if signed || !lossless {
      return Err(napi::Error::from_reason("BigInt out of u128 range"));
    }
    self.writer()?.serialize(val);
    Ok(())
  }

  // --- floating point scalars ----------------------------------------------

  // f32 has no `FromNapiValue` (JS numbers are doubles), so accept an `f64` and
  // narrow. The caller opted into f32 wire width by choosing this method.
  #[napi]
  pub fn f32(&mut self, value: f64) -> napi::Result<()> {
    self.writer()?.serialize(value as f32);
    Ok(())
  }

  #[napi]
  pub fn f64(&mut self, value: f64) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  #[napi]
  pub fn bool(&mut self, value: bool) -> napi::Result<()> {
    self.writer()?.serialize(value);
    Ok(())
  }

  // --- strings & byte blobs -------------------------------------------------

  #[napi]
  pub fn string(&mut self, value: String) -> napi::Result<()> {
    self.writer()?.serialize(value.as_str());
    Ok(())
  }

  /// Serialize a byte blob (LEB128 length prefix + raw bytes). Wire-compatible
  /// with a `Vec<u8>` / `ZBytes`.
  #[napi]
  pub fn bytes(&mut self, value: Uint8Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  /// Serialize a sequence of strings (LEB128 count + each string). Wire-
  /// compatible with a `Vec<String>`.
  #[napi]
  pub fn string_array(&mut self, value: Vec<String>) -> napi::Result<()> {
    self.writer()?.serialize(value.as_slice());
    Ok(())
  }

  // --- primitive typed arrays (LEB128 count + little-endian bulk) -----------

  #[napi]
  pub fn int8_array(&mut self, value: Int8Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn int16_array(&mut self, value: Int16Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn int32_array(&mut self, value: Int32Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn uint16_array(&mut self, value: Uint16Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn uint32_array(&mut self, value: Uint32Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn float32_array(&mut self, value: Float32Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn float64_array(&mut self, value: Float64Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn big_int64_array(&mut self, value: BigInt64Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  #[napi]
  pub fn big_uint64_array(&mut self, value: BigUint64Array) -> napi::Result<()> {
    self.writer()?.serialize(value.as_ref());
    Ok(())
  }

  // --- LEB128 varint --------------------------------------------------------

  /// Serialize a `usize` as an LEB128 variable-length integer. Used as the
  /// length/count prefix for hand-rolled sequences, maps, and sets.
  #[napi]
  pub fn varint(&mut self, value: BigInt) -> napi::Result<()> {
    let (signed, val, lossless) = value.get_u64();
    if signed || !lossless {
      return Err(napi::Error::from_reason("varint out of usize range"));
    }
    self.writer()?.serialize(VarInt(val as usize));
    Ok(())
  }

  /// Consume the serializer and return the serialized [`Bytes`]. Throws if the
  /// serializer was already finished.
  #[napi]
  pub fn finish(&mut self) -> napi::Result<Bytes> {
    let inner = self
      .inner
      .take()
      .ok_or_else(|| napi::Error::from_reason("serializer already finished"))?;
    Ok(Bytes::from_inner(inner.finish()))
  }
}

impl Default for Serializer {
  fn default() -> Self {
    Self::new()
  }
}

/// Streaming deserializer implementing the Zenoh serialization format.
///
/// Read values in the same order they were written. Use [`ZDeserializer::done`]
/// to check whether the buffer is fully consumed.
#[napi]
pub struct Deserializer {
  // Owning self-referential pair. `reader` borrows from `owner`; declaring it
  // BEFORE `owner` makes it drop first. A live read cursor cannot be modeled by
  // the replay strategy used for pure-input builders (there is no public seek to
  // re-skip consumed bytes), so this is the deliberate owning self-ref exception.
  //
  // SAFETY INVARIANTS (upheld below): `owner` is boxed (stable heap address) and
  // is never moved or mutated while `reader` is alive, so the `'static` borrow
  // never dangles. The `unsafe` is confined to the constructor.
  reader: ZDe<'static>,
  _owner: Box<ZBytes>,
}

#[napi]
impl Deserializer {
  #[napi(constructor)]
  pub fn new(data: &Bytes) -> Self {
    let owner = Box::new(data.clone_inner());
    // SAFETY: `owner` is heap-allocated, giving the `ZBytes` a stable address.
    // It is never moved or mutated for the lifetime of `reader`, and `reader` is
    // dropped before `owner` (field declaration order), so the extended `'static`
    // borrow is sound and fully contained within this struct.
    let reader = ZDe::new(unsafe { &*(owner.as_ref() as *const ZBytes) });
    Self {
      reader,
      _owner: owner,
    }
  }

  /// `true` when there is no data left to deserialize.
  #[napi(getter)]
  pub fn done(&self) -> bool {
    self.reader.done()
  }

  // --- integer scalars representable as a JS `number` -----------------------

  #[napi]
  pub fn i8(&mut self) -> napi::Result<i8> {
    self.reader.deserialize::<i8>().map_err(de_err)
  }

  #[napi]
  pub fn i16(&mut self) -> napi::Result<i16> {
    self.reader.deserialize::<i16>().map_err(de_err)
  }

  #[napi]
  pub fn i32(&mut self) -> napi::Result<i32> {
    self.reader.deserialize::<i32>().map_err(de_err)
  }

  #[napi]
  pub fn u8(&mut self) -> napi::Result<u8> {
    self.reader.deserialize::<u8>().map_err(de_err)
  }

  #[napi]
  pub fn u16(&mut self) -> napi::Result<u16> {
    self.reader.deserialize::<u16>().map_err(de_err)
  }

  #[napi]
  pub fn u32(&mut self) -> napi::Result<u32> {
    self.reader.deserialize::<u32>().map_err(de_err)
  }

  // --- 64/128-bit integer scalars carried as `BigInt` -----------------------

  #[napi]
  pub fn i64(&mut self) -> napi::Result<BigInt> {
    self
      .reader
      .deserialize::<i64>()
      .map(BigInt::from)
      .map_err(de_err)
  }

  #[napi]
  pub fn u64(&mut self) -> napi::Result<BigInt> {
    self
      .reader
      .deserialize::<u64>()
      .map(BigInt::from)
      .map_err(de_err)
  }

  #[napi]
  pub fn i128(&mut self) -> napi::Result<BigInt> {
    self
      .reader
      .deserialize::<i128>()
      .map(BigInt::from)
      .map_err(de_err)
  }

  #[napi]
  pub fn u128(&mut self) -> napi::Result<BigInt> {
    self
      .reader
      .deserialize::<u128>()
      .map(BigInt::from)
      .map_err(de_err)
  }

  // --- floating point scalars ----------------------------------------------

  #[napi]
  pub fn f32(&mut self) -> napi::Result<f32> {
    self.reader.deserialize::<f32>().map_err(de_err)
  }

  #[napi]
  pub fn f64(&mut self) -> napi::Result<f64> {
    self.reader.deserialize::<f64>().map_err(de_err)
  }

  #[napi]
  pub fn bool(&mut self) -> napi::Result<bool> {
    self.reader.deserialize::<bool>().map_err(de_err)
  }

  // --- strings & byte blobs -------------------------------------------------

  #[napi]
  pub fn string(&mut self) -> napi::Result<String> {
    self.reader.deserialize::<String>().map_err(de_err)
  }

  #[napi]
  pub fn bytes(&mut self) -> napi::Result<Uint8Array> {
    self
      .reader
      .deserialize::<Vec<u8>>()
      .map(Uint8Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn string_array(&mut self) -> napi::Result<Vec<String>> {
    self.reader.deserialize::<Vec<String>>().map_err(de_err)
  }

  // --- primitive typed arrays ----------------------------------------------

  #[napi]
  pub fn int8_array(&mut self) -> napi::Result<Int8Array> {
    self
      .reader
      .deserialize::<Vec<i8>>()
      .map(Int8Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn int16_array(&mut self) -> napi::Result<Int16Array> {
    self
      .reader
      .deserialize::<Vec<i16>>()
      .map(Int16Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn int32_array(&mut self) -> napi::Result<Int32Array> {
    self
      .reader
      .deserialize::<Vec<i32>>()
      .map(Int32Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn uint16_array(&mut self) -> napi::Result<Uint16Array> {
    self
      .reader
      .deserialize::<Vec<u16>>()
      .map(Uint16Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn uint32_array(&mut self) -> napi::Result<Uint32Array> {
    self
      .reader
      .deserialize::<Vec<u32>>()
      .map(Uint32Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn float32_array(&mut self) -> napi::Result<Float32Array> {
    self
      .reader
      .deserialize::<Vec<f32>>()
      .map(Float32Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn float64_array(&mut self) -> napi::Result<Float64Array> {
    self
      .reader
      .deserialize::<Vec<f64>>()
      .map(Float64Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn big_int64_array(&mut self) -> napi::Result<BigInt64Array> {
    self
      .reader
      .deserialize::<Vec<i64>>()
      .map(BigInt64Array::new)
      .map_err(de_err)
  }

  #[napi]
  pub fn big_uint64_array(&mut self) -> napi::Result<BigUint64Array> {
    self
      .reader
      .deserialize::<Vec<u64>>()
      .map(BigUint64Array::new)
      .map_err(de_err)
  }

  // --- LEB128 varint --------------------------------------------------------

  /// Deserialize an LEB128 variable-length `usize` (the count prefix written by
  /// [`ZSerializer::serialize_varint`]).
  #[napi]
  pub fn varint(&mut self) -> napi::Result<BigInt> {
    self
      .reader
      .deserialize::<VarInt<usize>>()
      .map(|v| BigInt::from(v.0 as u64))
      .map_err(de_err)
  }
}
