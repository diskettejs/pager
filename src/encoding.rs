use napi_derive::napi;
use zenoh::bytes::Encoding as ZEncoding;

#[napi]
pub struct Encoding {
  inner: ZEncoding,
}

impl Encoding {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZEncoding) -> Self {
    Encoding { inner }
  }
}

#[napi]
impl Encoding {
  #[napi(factory)]
  pub fn default() -> Self {
    Encoding {
      inner: ZEncoding::default(),
    }
  }

  #[napi(factory)]
  pub fn from(value: String) -> Self {
    Encoding {
      inner: ZEncoding::from(value),
    }
  }

  #[napi(factory)]
  pub fn zenoh_bytes() -> Self {
    Self::from_inner(ZEncoding::ZENOH_BYTES)
  }

  #[napi(factory)]
  pub fn zenoh_string() -> Self {
    Self::from_inner(ZEncoding::ZENOH_STRING)
  }

  #[napi(factory)]
  pub fn zenoh_serialized() -> Self {
    Self::from_inner(ZEncoding::ZENOH_SERIALIZED)
  }

  #[napi(factory)]
  pub fn application_octet_stream() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_OCTET_STREAM)
  }

  #[napi(factory)]
  pub fn text_plain() -> Self {
    Self::from_inner(ZEncoding::TEXT_PLAIN)
  }

  #[napi(factory)]
  pub fn application_json() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_JSON)
  }

  #[napi(factory)]
  pub fn text_json() -> Self {
    Self::from_inner(ZEncoding::TEXT_JSON)
  }

  #[napi(factory)]
  pub fn application_cdr() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_CDR)
  }

  #[napi(factory)]
  pub fn application_cbor() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_CBOR)
  }

  #[napi(factory)]
  pub fn application_yaml() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_YAML)
  }

  #[napi(factory)]
  pub fn text_yaml() -> Self {
    Self::from_inner(ZEncoding::TEXT_YAML)
  }

  #[napi(factory)]
  pub fn text_json5() -> Self {
    Self::from_inner(ZEncoding::TEXT_JSON5)
  }

  #[napi(factory)]
  pub fn application_python_serialized_object() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_PYTHON_SERIALIZED_OBJECT)
  }

  #[napi(factory)]
  pub fn application_protobuf() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_PROTOBUF)
  }

  #[napi(factory)]
  pub fn application_java_serialized_object() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_JAVA_SERIALIZED_OBJECT)
  }

  #[napi(factory)]
  pub fn application_openmetrics_text() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_OPENMETRICS_TEXT)
  }

  #[napi(factory)]
  pub fn image_png() -> Self {
    Self::from_inner(ZEncoding::IMAGE_PNG)
  }

  #[napi(factory)]
  pub fn image_jpeg() -> Self {
    Self::from_inner(ZEncoding::IMAGE_JPEG)
  }

  #[napi(factory)]
  pub fn image_gif() -> Self {
    Self::from_inner(ZEncoding::IMAGE_GIF)
  }

  #[napi(factory)]
  pub fn image_bmp() -> Self {
    Self::from_inner(ZEncoding::IMAGE_BMP)
  }

  #[napi(factory)]
  pub fn image_webp() -> Self {
    Self::from_inner(ZEncoding::IMAGE_WEBP)
  }

  #[napi(factory)]
  pub fn application_xml() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_XML)
  }

  #[napi(factory)]
  pub fn application_x_www_form_urlencoded() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_X_WWW_FORM_URLENCODED)
  }

  #[napi(factory)]
  pub fn text_html() -> Self {
    Self::from_inner(ZEncoding::TEXT_HTML)
  }

  #[napi(factory)]
  pub fn text_xml() -> Self {
    Self::from_inner(ZEncoding::TEXT_XML)
  }

  #[napi(factory)]
  pub fn text_css() -> Self {
    Self::from_inner(ZEncoding::TEXT_CSS)
  }

  #[napi(factory)]
  pub fn text_javascript() -> Self {
    Self::from_inner(ZEncoding::TEXT_JAVASCRIPT)
  }

  #[napi(factory)]
  pub fn text_markdown() -> Self {
    Self::from_inner(ZEncoding::TEXT_MARKDOWN)
  }

  #[napi(factory)]
  pub fn text_csv() -> Self {
    Self::from_inner(ZEncoding::TEXT_CSV)
  }

  #[napi(factory)]
  pub fn application_sql() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_SQL)
  }

  #[napi(factory)]
  pub fn application_coap_payload() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_COAP_PAYLOAD)
  }

  #[napi(factory)]
  pub fn application_json_patch_json() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_JSON_PATCH_JSON)
  }

  #[napi(factory)]
  pub fn application_json_seq() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_JSON_SEQ)
  }

  #[napi(factory)]
  pub fn application_jsonpath() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_JSONPATH)
  }

  #[napi(factory)]
  pub fn application_jwt() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_JWT)
  }

  #[napi(factory)]
  pub fn application_mp4() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_MP4)
  }

  #[napi(factory)]
  pub fn application_soap_xml() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_SOAP_XML)
  }

  #[napi(factory)]
  pub fn application_yang() -> Self {
    Self::from_inner(ZEncoding::APPLICATION_YANG)
  }

  #[napi(factory)]
  pub fn audio_aac() -> Self {
    Self::from_inner(ZEncoding::AUDIO_AAC)
  }

  #[napi(factory)]
  pub fn audio_flac() -> Self {
    Self::from_inner(ZEncoding::AUDIO_FLAC)
  }

  #[napi(factory)]
  pub fn audio_mp4() -> Self {
    Self::from_inner(ZEncoding::AUDIO_MP4)
  }

  #[napi(factory)]
  pub fn audio_ogg() -> Self {
    Self::from_inner(ZEncoding::AUDIO_OGG)
  }

  #[napi(factory)]
  pub fn audio_vorbis() -> Self {
    Self::from_inner(ZEncoding::AUDIO_VORBIS)
  }

  #[napi(factory)]
  pub fn video_h261() -> Self {
    Self::from_inner(ZEncoding::VIDEO_H261)
  }

  #[napi(factory)]
  pub fn video_h263() -> Self {
    Self::from_inner(ZEncoding::VIDEO_H263)
  }

  #[napi(factory)]
  pub fn video_h264() -> Self {
    Self::from_inner(ZEncoding::VIDEO_H264)
  }

  #[napi(factory)]
  pub fn video_h265() -> Self {
    Self::from_inner(ZEncoding::VIDEO_H265)
  }

  #[napi(factory)]
  pub fn video_h266() -> Self {
    Self::from_inner(ZEncoding::VIDEO_H266)
  }

  #[napi(factory)]
  pub fn video_mp4() -> Self {
    Self::from_inner(ZEncoding::VIDEO_MP4)
  }

  #[napi(factory)]
  pub fn video_ogg() -> Self {
    Self::from_inner(ZEncoding::VIDEO_OGG)
  }

  #[napi(factory)]
  pub fn video_raw() -> Self {
    Self::from_inner(ZEncoding::VIDEO_RAW)
  }

  #[napi(factory)]
  pub fn video_vp8() -> Self {
    Self::from_inner(ZEncoding::VIDEO_VP8)
  }

  #[napi(factory)]
  pub fn video_vp9() -> Self {
    Self::from_inner(ZEncoding::VIDEO_VP9)
  }

  #[napi]
  pub fn to_string(&self) -> String {
    self.inner.to_string()
  }

  #[napi]
  pub fn with_schema(&self, value: String) -> Self {
    Encoding {
      inner: ZEncoding::from(self.inner.to_string()).with_schema(value),
    }
  }
}
