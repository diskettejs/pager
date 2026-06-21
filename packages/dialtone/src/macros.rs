//! Internal helper macros shared across the binding modules.

/// Fold the set fields of a napi options object onto the matching zenoh builder.
///
/// Every zenoh operation takes an `Option<…Options>` whose fields each map to a
/// builder setter of the same name, applied only when the field is `Some`.
/// Spelled out, that is dozens of near-identical
///
/// ```ignore
/// if let Some(v) = options.field {
///   builder = builder.field(/* convert v */);
/// }
/// ```
///
/// blocks. This macro collapses each to a single line. The field name doubles as
/// the builder method name; the optional `=> kind` suffix selects how the JS
/// value is converted before it reaches zenoh:
///
/// | suffix           | conversion                          | used for                   |
/// |------------------|-------------------------------------|----------------------------|
/// | *(none)*         | passed through unchanged            | `encoding`, `express`      |
/// | `=> into`        | `v.into()`                          | QoS enums, `Locality`      |
/// | `=> zbytes`      | `to_zbytes(v)`                      | `payload`, `attachment`    |
/// | `=> try_zenoh`   | `v.to_zenoh()?` (fallible)          | `timestamp`, `source_info` |
/// | `=> duration_ms` | `Duration::from_millis(v.into())`   | `timeout`                  |
/// | `=> from(T)`     | `<T>::from(v)`                      | `consolidation`            |
///
/// `builder` must be a `mut` binding and `options` the owned options struct;
/// fields are moved out, so any field the macro does not list (e.g. `handler`)
/// stays available afterwards. `try_zenoh` propagates with `?`, so the caller
/// must return `Result`.
macro_rules! apply_options {
  ($builder:ident, $options:ident, {
    $( $field:ident $(=> $kind:ident $(($ty:ty))? )? ),* $(,)?
  }) => {
    $(
      if let Some(value) = $options.$field {
        $builder = $builder.$field(
          $crate::macros::apply_options!(@conv value $(, $kind $(($ty))? )?)
        );
      }
    )*
  };

  // Per-field value conversions selected by the `=> kind` suffix above.
  (@conv $v:expr) => { $v };
  (@conv $v:expr, into) => { $v.into() };
  (@conv $v:expr, zbytes) => { $crate::bytes::to_zbytes($v) };
  (@conv $v:expr, try_zenoh) => { $v.to_zenoh()? };
  (@conv $v:expr, duration_ms) => { ::std::time::Duration::from_millis($v.into()) };
  (@conv $v:expr, from($ty:ty)) => { <$ty>::from($v) };
}

pub(crate) use apply_options;
