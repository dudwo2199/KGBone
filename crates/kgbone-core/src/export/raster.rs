//! 소프트웨어 삼각형 그리기(PNG 시퀀스용). 미리보기 렌더러(WebGL2)와 같은 그리기 목록(DrawItem)을 쓴다.
//!
//! 좌표: 월드 y는 위로, 이미지 y는 아래로. 픽셀 (px, py) = ((x − center_x)·scale + 폭/2, 높이/2 − (y − center_y)·scale).
//! 픽셀 중심 (i + 0.5, j + 0.5)이 삼각형 안이면 칠한다. 공유 변 위의 중심은 한 삼각형만 칠한다(겹쳐 두 번 섞이지 않게).
//!
//! 색 계산은 알파를 곱한(premultiplied) 0~1 실수로 한다.
//! - 텍스처: 쌍선형 샘플(가장자리는 늘림), 곱한 알파로 보간해 투명한 이웃의 색이 번지지 않게 한다.
//! - 틴트: 곱하지 않은 텍스처 색 s, 슬롯 색 c, 어두운 색 d에 대해
//!   결과색 = s·c.rgb + (1 − s)·d (어두운 색이 없으면 d = 0 → s·c.rgb), 결과 알파 = 텍스처 알파 · c.a.
//!   즉 텍스처의 검정 부분이 d, 흰 부분이 c가 되는 "검정 틴트".
//! - 블렌드(곱하지 않은 색 기준 함수 B를 원본-위(source-over) 합성에 넣은 W3C 합성식):
//!   normal  B = Cs, additive는 색을 더함, multiply B = Cs·Cb, screen B = Cs + Cb − Cs·Cb.
//!   알파는 모두 αs + αb·(1 − αs).
use super::Image;
use crate::model::BlendMode;
use crate::pose::DrawItem;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    /// 월드 좌표 → 픽셀 배율.
    pub scale: f32,
    /// 이미지 중심에 올 월드 좌표.
    pub center_x: f32,
    pub center_y: f32,
    /// 배경색 RGBA(투명이면 [0,0,0,0]).
    pub background: [u8; 4],
}

/// 그리기 목록을 이미지 한 장으로 그린다. `images`: 이미지 경로 → 픽셀. 이미지가 없는 항목은 건너뛴다.
pub fn render_pose(items: &[DrawItem], images: &HashMap<String, Image>, opts: &RenderOptions) -> Image {
    let (w, h) = (opts.width as usize, opts.height as usize);
    let bg = opts.background;
    let ba = bg[3] as f32 / 255.0;
    let bgp = [bg[0] as f32 / 255.0 * ba, bg[1] as f32 / 255.0 * ba, bg[2] as f32 / 255.0 * ba, ba];
    let mut canvas = Canvas { w, h, px: vec![bgp; w * h] };
    for item in items {
        let Some(tex) = images.get(&item.image) else { continue };
        if tex.width == 0 || tex.height == 0 || tex.rgba.len() != (tex.width * tex.height * 4) as usize {
            continue;
        }
        draw_item(&mut canvas, item, tex, opts);
    }
    canvas.into_image()
}

/// 곱한 알파 RGBA 실수 화면.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn into_image(self) -> Image {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut rgba = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.px {
            let a = p[3].clamp(0.0, 1.0);
            if a <= 0.0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                rgba.extend_from_slice(&[q(p[0] / a), q(p[1] / a), q(p[2] / a), q(a)]);
            }
        }
        Image { width: self.w as u32, height: self.h as u32, rgba }
    }
}

#[derive(Clone, Copy)]
struct Vtx {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
}

fn draw_item(c: &mut Canvas, item: &DrawItem, tex: &Image, opts: &RenderOptions) {
    let n = (item.positions.len() / 2).min(item.uvs.len() / 2);
    let (hw, hh) = (opts.width as f32 / 2.0, opts.height as f32 / 2.0);
    let verts: Vec<Vtx> = (0..n)
        .map(|i| Vtx {
            x: (item.positions[i * 2] - opts.center_x) * opts.scale + hw,
            y: hh - (item.positions[i * 2 + 1] - opts.center_y) * opts.scale,
            u: item.uvs[i * 2],
            v: item.uvs[i * 2 + 1],
        })
        .collect();
    for tri in item.indices.chunks_exact(3) {
        let (a, b, cc) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if a >= n || b >= n || cc >= n {
            continue;
        }
        fill_triangle(c, [verts[a], verts[b], verts[cc]], item, tex);
    }
}

/// 변 함수: a→b 변에 대해 p가 어느 쪽인지(넓이 두 배).
fn edge(a: Vtx, b: Vtx, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

/// 변 위(값 0)의 픽셀을 이 삼각형이 가질지. 이웃 삼각형은 같은 변을 반대 방향으로 지나므로 꼭 한쪽만 참이 된다.
fn owns_edge(a: Vtx, b: Vtx) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    dy > 0.0 || (dy == 0.0 && dx < 0.0)
}

fn fill_triangle(c: &mut Canvas, mut v: [Vtx; 3], item: &DrawItem, tex: &Image) {
    let mut area = edge(v[0], v[1], v[2].x, v[2].y);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    if area < 0.0 {
        v.swap(1, 2);
        area = -area;
    }
    let minx = v.iter().map(|p| p.x).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
    let maxx = (v.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max).ceil().min(c.w as f32)).max(0.0) as usize;
    let miny = v.iter().map(|p| p.y).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
    let maxy = (v.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max).ceil().min(c.h as f32)).max(0.0) as usize;
    let own = [owns_edge(v[1], v[2]), owns_edge(v[2], v[0]), owns_edge(v[0], v[1])];
    let inside = |e: f32, owned: bool| e > 0.0 || (e == 0.0 && owned);
    for py in miny..maxy {
        let fy = py as f32 + 0.5;
        for px in minx..maxx {
            let fx = px as f32 + 0.5;
            let e0 = edge(v[1], v[2], fx, fy);
            let e1 = edge(v[2], v[0], fx, fy);
            let e2 = edge(v[0], v[1], fx, fy);
            if !(inside(e0, own[0]) && inside(e1, own[1]) && inside(e2, own[2])) {
                continue;
            }
            let (l0, l1, l2) = (e0 / area, e1 / area, e2 / area);
            let u = l0 * v[0].u + l1 * v[1].u + l2 * v[2].u;
            let tv = l0 * v[0].v + l1 * v[1].v + l2 * v[2].v;
            let src = shade(sample(tex, u, tv), item);
            let dst = &mut c.px[py * c.w + px];
            *dst = blend(item.blend, src, *dst);
        }
    }
}

/// 쌍선형 샘플 → 곱한 알파 RGBA(0~1). 텍셀 중심은 ((i + 0.5)/폭, (j + 0.5)/높이).
fn sample(tex: &Image, u: f32, v: f32) -> [f32; 4] {
    let (tw, th) = (tex.width as i64, tex.height as i64);
    let fx = u * tw as f32 - 0.5;
    let fy = v * th as f32 - 0.5;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (ax, ay) = (fx - x0, fy - y0);
    let texel = |x: i64, y: i64| -> [f32; 4] {
        let (x, y) = (x.clamp(0, tw - 1), y.clamp(0, th - 1));
        let i = ((y * tw + x) * 4) as usize;
        let a = tex.rgba[i + 3] as f32 / 255.0;
        [tex.rgba[i] as f32 / 255.0 * a, tex.rgba[i + 1] as f32 / 255.0 * a, tex.rgba[i + 2] as f32 / 255.0 * a, a]
    };
    let (x0, y0) = (x0 as i64, y0 as i64);
    let (p00, p10, p01, p11) = (texel(x0, y0), texel(x0 + 1, y0), texel(x0, y0 + 1), texel(x0 + 1, y0 + 1));
    let mut out = [0.0; 4];
    for k in 0..4 {
        let top = p00[k] + (p10[k] - p00[k]) * ax;
        let bot = p01[k] + (p11[k] - p01[k]) * ax;
        out[k] = top + (bot - top) * ay;
    }
    out
}

/// 틴트 적용(곱한 알파 그대로 계산).
/// 곱하지 않은 식 rgb = s·c + (1 − s)·d, α = ta·c.a 에 α를 곱하면
/// rgb·α = (S·c + (ta − S)·d)·c.a  (S = s·ta, 곱한 텍스처 색).
fn shade(t: [f32; 4], item: &DrawItem) -> [f32; 4] {
    let c = item.color;
    let d = item.dark.unwrap_or([0.0; 3]);
    let a = t[3] * c[3];
    let mut out = [0.0, 0.0, 0.0, a];
    for k in 0..3 {
        out[k] = (t[k] * c[k] + (t[3] - t[k]) * d[k]) * c[3];
    }
    out
}

/// 곱한 알파 원본 s를 바탕 b 위에 합성한다.
fn blend(mode: BlendMode, s: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let (sa, ba) = (s[3], b[3]);
    let alpha = sa + ba * (1.0 - sa);
    let mut out = [0.0, 0.0, 0.0, alpha];
    for k in 0..3 {
        let (cs, cb) = (s[k], b[k]);
        out[k] = match mode {
            BlendMode::Normal => cs + cb * (1.0 - sa),
            // 색은 그대로 더한다(빛이 겹치는 효과). 1을 넘으면 저장할 때 잘린다.
            BlendMode::Additive => cs + cb,
            // co = cs·cb + cs·(1 − αb) + cb·(1 − αs)
            BlendMode::Multiply => cs * cb + cs * (1.0 - ba) + cb * (1.0 - sa),
            // co = cs + cb − cs·cb
            BlendMode::Screen => cs + cb - cs * cb,
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> Image {
        Image { width: w, height: h, rgba: c.iter().copied().cycle().take((w * h * 4) as usize).collect() }
    }

    /// 월드 (x0,y0)~(x1,y1) 사각형, 이미지 전체를 붙인다. 정점: 왼쪽 아래, 오른쪽 아래, 오른쪽 위, 왼쪽 위.
    fn quad(image: &str, x0: f32, y0: f32, x1: f32, y1: f32, blend: BlendMode) -> DrawItem {
        DrawItem {
            slot: "s".into(),
            attachment: "a".into(),
            image: image.into(),
            blend,
            color: [1.0; 4],
            dark: None,
            positions: vec![x0, y0, x1, y0, x1, y1, x0, y1],
            // v는 아래로: 왼쪽 아래 정점 = (0, 1).
            uvs: vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
            indices: vec![0, 1, 2, 2, 3, 0],
        }
    }

    fn px(img: &Image, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.width + x) * 4) as usize;
        [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2], img.rgba[i + 3]]
    }

    fn opts(bg: [u8; 4]) -> RenderOptions {
        RenderOptions { width: 20, height: 20, scale: 1.0, center_x: 0.0, center_y: 0.0, background: bg }
    }

    #[test]
    fn k02_raster_solid_quad_fills_its_area() {
        let mut images = HashMap::new();
        images.insert("red".to_string(), solid(4, 4, [255, 0, 0, 255]));
        // 월드 x 0~5, y 0~5 → 픽셀 x 10~15, y 5~10(위쪽 행부터: 월드 y=5가 픽셀 y=5).
        let img = render_pose(&[quad("red", 0.0, 0.0, 5.0, 5.0, BlendMode::Normal)], &images, &opts([0, 0, 255, 255]));
        assert_eq!((img.width, img.height), (20, 20));
        for y in 0..20 {
            for x in 0..20 {
                let want = if (10..15).contains(&x) && (5..10).contains(&y) { [255, 0, 0, 255] } else { [0, 0, 255, 255] };
                assert_eq!(px(&img, x, y), want, "({x},{y})");
            }
        }
        // 없는 이미지는 건너뛴다.
        let img = render_pose(&[quad("none", 0.0, 0.0, 5.0, 5.0, BlendMode::Normal)], &images, &opts([0; 4]));
        assert!(img.rgba.iter().all(|&b| b == 0));
    }

    #[test]
    fn k02_raster_texture_orientation_and_tint() {
        // 2×2 텍스처: 위 줄 [빨강, 초록], 아래 줄 [파랑, 흰색]. 월드 −10~10 전체(20×20 픽셀)에 붙인다.
        let tex = Image { width: 2, height: 2, rgba: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255] };
        let mut images = HashMap::new();
        images.insert("t".to_string(), tex);
        let item = quad("t", -10.0, -10.0, 10.0, 10.0, BlendMode::Normal);
        let img = render_pose(&[item.clone()], &images, &opts([0; 4]));
        // 픽셀 (0,0) 중심 → u = 0.025, 텍셀 좌표 −0.45 → 가장자리 늘림으로 빨강 그대로. 오른쪽 위 = 초록, 왼쪽 아래 = 파랑.
        assert_eq!(px(&img, 0, 0), [255, 0, 0, 255]);
        assert_eq!(px(&img, 19, 0), [0, 255, 0, 255]);
        assert_eq!(px(&img, 0, 19), [0, 0, 255, 255]);
        assert_eq!(px(&img, 19, 19), [255, 255, 255, 255]);
        // 가운데 (10,10) 중심 → u = v = 0.525, 텍셀 좌표 0.55 → 네 색을 0.45·0.45 … 섞음:
        // r = 빨강(0.45·0.45=0.2025) + 흰색(0.55·0.55=0.3025) = 0.505 → 129
        let c = px(&img, 10, 10);
        assert_eq!(c[0], 129);

        // 색 곱: 흰 칸(19,19)에 color (0.5, 1, 0, 1) → (128, 255, 0).
        let mut tinted = item.clone();
        tinted.color = [0.5, 1.0, 0.0, 1.0];
        assert_eq!(px(&render_pose(&[tinted], &images, &opts([0; 4])), 19, 19), [128, 255, 0, 255]);
        // 검정 틴트: 빨강 칸(0,0) s = (1,0,0), c = 흰색, d = (0, 0.5, 1) → rgb = s + (1 − s)·d = (1, 0.5, 1).
        let mut dark = item;
        dark.dark = Some([0.0, 0.5, 1.0]);
        assert_eq!(px(&render_pose(&[dark], &images, &opts([0; 4])), 0, 0), [255, 128, 255, 255]);
    }

    #[test]
    fn k02_raster_blend_modes() {
        let mut images = HashMap::new();
        images.insert("half".to_string(), solid(2, 2, [100, 50, 200, 128]));
        let q = |b| quad("half", -10.0, -10.0, 10.0, 10.0, b);
        let bg = [40, 80, 120, 255];
        // additive: 색 더함. 원본(곱한 알파) = 100·α … α = 128/255 ≈ 0.502
        //   r = 40 + 100·0.502 = 90.2 → 90, g = 80 + 25.1 = 105, b = 120 + 100.4 = 220.4 → 220
        assert_eq!(px(&render_pose(&[q(BlendMode::Additive)], &images, &opts(bg)), 3, 3), [90, 105, 220, 255]);
        // normal: r = 100·0.502 + 40·0.498 = 70.1 → 70, g = 25.1 + 39.84 = 64.9 → 65, b = 100.4 + 59.8 = 160.2 → 160
        assert_eq!(px(&render_pose(&[q(BlendMode::Normal)], &images, &opts(bg)), 3, 3), [70, 65, 160, 255]);
        // multiply(바탕 불투명): co = cs·cb + cb·(1 − αs) → r/255 = 0.392·0.502·0.157 + 0.157·0.498
        //   = 0.0309 + 0.0781 = 0.1090 → 27.8 → 28
        assert_eq!(px(&render_pose(&[q(BlendMode::Multiply)], &images, &opts(bg)), 3, 3)[0], 28);
        // screen: co = cs + cb − cs·cb → r/255 = 0.1969 + 0.1569 − 0.0309 = 0.3229 → 82.3 → 82
        assert_eq!(px(&render_pose(&[q(BlendMode::Screen)], &images, &opts(bg)), 3, 3)[0], 82);
        // 투명 바탕 위 additive는 원본 그대로: 곱하지 않은 색 100, 알파 128.
        assert_eq!(px(&render_pose(&[q(BlendMode::Additive)], &images, &opts([0; 4])), 3, 3), [100, 50, 200, 128]);
    }

    #[test]
    fn k02_raster_shared_edge_drawn_once() {
        // 반투명 사각형의 대각선(공유 변) 위 픽셀도 한 번만 섞여 다른 픽셀과 같아야 한다.
        let mut images = HashMap::new();
        images.insert("half".to_string(), solid(2, 2, [255, 255, 255, 128]));
        let img = render_pose(&[quad("half", -10.0, -10.0, 10.0, 10.0, BlendMode::Additive)], &images, &opts([0, 0, 0, 255]));
        let first = px(&img, 0, 0);
        assert!(img.rgba.chunks(4).all(|p| p == first), "모든 픽셀이 한 번씩");
    }
}
