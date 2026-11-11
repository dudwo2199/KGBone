//! 내보내기: 텍스처 패킹(.atlas + PNG), PNG 시퀀스(소프트웨어 그리기).
//! 이미지 파일 읽기·쓰기는 호출하는 쪽(앱 본체)이 하고, 여기서는 픽셀 데이터만 다룬다.
pub mod base64;
mod bundle;
mod checksum;
mod deflate;
mod deflate_tables;
mod inflate;
mod pack;
mod png_io;
mod raster;
mod sequence;

pub use bundle::{export_png_sequence, export_spine_bundle, Bundle, ImageSource, PngSequenceOptions, SpineBundleOptions};
pub use pack::{pack_images, PackInput, PackOptions, PackResult, PackedPage};
pub use png_io::{decode_png, encode_png, png_size, Image};
pub use raster::{render_pose, RenderOptions};
pub use sequence::{sequence_times, SequenceOptions};
