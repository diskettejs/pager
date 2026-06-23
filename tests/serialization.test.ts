import { describe, expect, test } from 'vitest'
import { Bytes, Deserializer, Serializer } from '../index.js'

/** Serialize via `fn`, return the produced bytes as a plain number[]. */
function bytesOf(fn: (s: Serializer) => void): number[] {
  const s = new Serializer()
  fn(s)
  return Array.from(s.finish().toBytes())
}

/** A deserializer reading the given raw bytes. */
function reader(bytes: number[]): Deserializer {
  return new Deserializer(Bytes.fromBytes(new Uint8Array(bytes)))
}

describe('Serializer wire format (golden bytes from zenoh-ext binary_format)', () => {
  test('i32', () => {
    expect(bytesOf((s) => s.i32(1234566))).toEqual([134, 214, 18, 0])
    expect(bytesOf((s) => s.i32(-49245))).toEqual([163, 63, 255, 255])
  })

  test('string', () => {
    expect(bytesOf((s) => s.string('test'))).toEqual([4, 116, 101, 115, 116])
  })

  test('tuple (u16, f32, string) = sequential calls, no length prefix', () => {
    const out = bytesOf((s) => {
      s.u16(500)
      s.f32(1234.0)
      s.string('test')
    })
    expect(out).toEqual([244, 1, 0, 64, 154, 68, 4, 116, 101, 115, 116])
  })

  test('Vec<i64> via BigInt64Array (LEB128 count + LE bulk)', () => {
    const out = bytesOf((s) =>
      s.bigInt64Array(new BigInt64Array([-100n, 500n, 100000n, -20000000n])),
    )
    expect(out).toEqual([
      4, 156, 255, 255, 255, 255, 255, 255, 255, 244, 1, 0, 0, 0, 0, 0, 0, 160, 134, 1, 0, 0, 0, 0,
      0, 0, 211, 206, 254, 255, 255, 255, 255,
    ])
  })

  test('Vec<(string, i16)> via the count + element protocol', () => {
    const out = bytesOf((s) => {
      s.varint(2n)
      s.string('s1')
      s.i16(10)
      s.string('s2')
      s.i16(-10000)
    })
    expect(out).toEqual([2, 2, 115, 49, 10, 0, 2, 115, 50, 240, 216])
  })
})

describe('Deserializer reads upstream golden bytes', () => {
  test('Vec<i64>', () => {
    const d = reader([
      4, 156, 255, 255, 255, 255, 255, 255, 255, 244, 1, 0, 0, 0, 0, 0, 0, 160, 134, 1, 0, 0, 0, 0,
      0, 0, 211, 206, 254, 255, 255, 255, 255,
    ])
    expect(d.bigInt64Array()).toEqual(new BigInt64Array([-100n, 500n, 100000n, -20000000n]))
    expect(d.done).toBe(true)
  })

  test('Vec<(string, i16)> read back element by element', () => {
    const d = reader([2, 2, 115, 49, 10, 0, 2, 115, 50, 240, 216])
    const count = Number(d.varint())
    expect(count).toBe(2)
    expect(d.string()).toBe('s1')
    expect(d.i16()).toBe(10)
    expect(d.string()).toBe('s2')
    expect(d.i16()).toBe(-10000)
    expect(d.done).toBe(true)
  })
})

describe('round trips', () => {
  test('integer scalars (number)', () => {
    const s = new Serializer()
    s.i8(-12)
    s.i16(-1234)
    s.i32(-123456)
    s.u8(200)
    s.u16(60000)
    s.u32(4000000000)
    const d = new Deserializer(s.finish())
    expect(d.i8()).toBe(-12)
    expect(d.i16()).toBe(-1234)
    expect(d.i32()).toBe(-123456)
    expect(d.u8()).toBe(200)
    expect(d.u16()).toBe(60000)
    expect(d.u32()).toBe(4000000000)
    expect(d.done).toBe(true)
  })

  test('64/128-bit scalars (BigInt)', () => {
    const s = new Serializer()
    s.i64(-9223372036854775808n)
    s.u64(18446744073709551615n)
    s.i128(-170141183460469231731687303715884105728n)
    s.u128(340282366920938463463374607431768211455n)
    const d = new Deserializer(s.finish())
    expect(d.i64()).toBe(-9223372036854775808n)
    expect(d.u64()).toBe(18446744073709551615n)
    expect(d.i128()).toBe(-170141183460469231731687303715884105728n)
    expect(d.u128()).toBe(340282366920938463463374607431768211455n)
    expect(d.done).toBe(true)
  })

  test('floats, bool, string, bytes', () => {
    const s = new Serializer()
    s.f32(1.5)
    s.f64(0.123456789)
    s.bool(true)
    s.bool(false)
    s.string('hello world')
    s.bytes(new Uint8Array([1, 2, 3, 255]))
    const d = new Deserializer(s.finish())
    expect(d.f32()).toBe(1.5)
    expect(d.f64()).toBe(0.123456789)
    expect(d.bool()).toBe(true)
    expect(d.bool()).toBe(false)
    expect(d.string()).toBe('hello world')
    expect(d.bytes()).toEqual(new Uint8Array([1, 2, 3, 255]))
    expect(d.done).toBe(true)
  })

  test('string array', () => {
    const s = new Serializer()
    s.stringArray(['a', 'bc', 'def'])
    const d = new Deserializer(s.finish())
    expect(d.stringArray()).toEqual(['a', 'bc', 'def'])
    expect(d.done).toBe(true)
  })

  test('primitive typed arrays', () => {
    const s = new Serializer()
    s.int8Array(new Int8Array([-1, 2, -3]))
    s.int16Array(new Int16Array([-1000, 1000]))
    s.int32Array(new Int32Array([-100000, 100000]))
    s.uint16Array(new Uint16Array([0, 65535]))
    s.uint32Array(new Uint32Array([0, 4000000000]))
    s.float32Array(new Float32Array([1.5, -2.25]))
    s.float64Array(new Float64Array([1.25, -3.5]))
    s.bigUint64Array(new BigUint64Array([0n, 18446744073709551615n]))
    const d = new Deserializer(s.finish())
    expect(d.int8Array()).toEqual(new Int8Array([-1, 2, -3]))
    expect(d.int16Array()).toEqual(new Int16Array([-1000, 1000]))
    expect(d.int32Array()).toEqual(new Int32Array([-100000, 100000]))
    expect(d.uint16Array()).toEqual(new Uint16Array([0, 65535]))
    expect(d.uint32Array()).toEqual(new Uint32Array([0, 4000000000]))
    expect(d.float32Array()).toEqual(new Float32Array([1.5, -2.25]))
    expect(d.float64Array()).toEqual(new Float64Array([1.25, -3.5]))
    expect(d.bigUint64Array()).toEqual(new BigUint64Array([0n, 18446744073709551615n]))
    expect(d.done).toBe(true)
  })

  test('map via count + key/value protocol', () => {
    const map = new Map<string, number>([
      ['one', 1],
      ['two', 2],
    ])
    const s = new Serializer()
    s.varint(BigInt(map.size))
    for (const [k, v] of map) {
      s.string(k)
      s.i32(v)
    }
    const d = new Deserializer(s.finish())
    const out = new Map<string, number>()
    const count = Number(d.varint())
    for (let i = 0; i < count; i++) {
      out.set(d.string(), d.i32())
    }
    expect(out).toEqual(map)
    expect(d.done).toBe(true)
  })
})

describe('lifecycle & error handling', () => {
  test('serializer throws after finish', () => {
    const s = new Serializer()
    s.i32(1)
    s.finish()
    expect(() => s.i32(2)).toThrow()
    expect(() => s.finish()).toThrow()
  })

  test('BigInt out-of-range throws', () => {
    const s = new Serializer()
    expect(() => s.u64(-1n)).toThrow()
    expect(() => s.i64(9223372036854775808n)).toThrow()
  })

  test('deserializing past the end throws', () => {
    const d = reader([1, 0, 0, 0])
    expect(d.i32()).toBe(1)
    expect(d.done).toBe(true)
    expect(() => d.i32()).toThrow()
  })
})
