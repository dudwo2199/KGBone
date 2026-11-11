//! 텍스처 패킹: 공백 제거 → 큰 것부터 정렬 → MaxRects(짧은 변 기준 최적) 배치 → 페이지 합성.
//!
//! atlas 규칙(docs/SOURCES.md S2):
//! - offsets = 잘라낸 왼쪽·아래 공백, 원래 폭·높이.
//! - 회전한 영역은 페이지에 반시계 90도로 돌려 저장하고 degrees = 90. 영역 폭·높이는 원래 방향 값.
use super::Image;
use crate::format::atlas::{Atlas, AtlasPage, AtlasRegion};
use crate::{CoreError, Result};

/// 패킹할 이미지 하나.
#[derive(Clone, Debug)]
pub struct PackInput {
    /// 영역 이름(첨부물 경로).
    pub name: String,
    pub image: Image,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PackOptions {
    pub max_width: u32,
    pub max_height: u32,
    /// 영역 사이 여백(픽셀).
    pub padding: u32,
    /// 투명한 가장자리 잘라내기.
    pub strip_whitespace: bool,
    /// 더 잘 들어가면 90도 회전 허용.
    pub rotation: bool,
    /// 크기를 2의 거듭제곱으로.
    pub power_of_two: bool,
    pub pma: bool,
    /// 페이지 이미지 이름 앞부분(예: `skeleton` → `skeleton.png`, `skeleton_2.png`).
    pub page_name: String,
}

impl Default for PackOptions {
    fn default() -> Self {
        PackOptions {
            max_width: 2048,
            max_height: 2048,
            padding: 2,
            strip_whitespace: true,
            rotation: false,
            power_of_two: true,
            pma: false,
            page_name: "skeleton".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackedPage {
    pub file_name: String,
    pub image: Image,
}

#[derive(Clone, Debug)]
pub struct PackResult {
    pub atlas: Atlas,
    pub pages: Vec<PackedPage>,
}

/// 원본 이미지 안의 잘라 쓸 사각형.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trim {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// 알파가 0이 아닌 픽셀을 모두 감싸는 사각형. 완전히 투명하면 왼쪽 위 1×1.
fn trim_rect(img: &Image) -> Trim {
    let (w, h) = (img.width, img.height);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if img.rgba[((y * w + x) * 4 + 3) as usize] != 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return Trim { x: 0, y: 0, w: 1.min(w), h: 1.min(h) };
    }
    Trim { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// 빈 사각형 목록으로 남은 자리를 기억하는 배치기(MaxRects).
struct MaxRects {
    free: Vec<Rect>,
    /// 실제로 쓴 범위(오른쪽·아래 끝).
    used_w: u32,
    used_h: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Rect {
    fn contains(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.x + o.w <= self.x + self.w && o.y + o.h <= self.y + self.h
    }
    fn intersects(&self, o: &Rect) -> bool {
        o.x < self.x + self.w && self.x < o.x + o.w && o.y < self.y + self.h && self.y < o.y + o.h
    }
}

impl MaxRects {
    fn new(w: u32, h: u32) -> Self {
        MaxRects { free: vec![Rect { x: 0, y: 0, w, h }], used_w: 0, used_h: 0 }
    }

    /// (w, h) 자리를 찾는다. 남는 짧은 변이 가장 작은 곳, 같으면 긴 변. → (x, y, 회전 여부, 점수)
    fn find(&self, w: u32, h: u32, allow_rotate: bool) -> Option<(u32, u32, bool, (u32, u32))> {
        let mut best: Option<(u32, u32, bool, (u32, u32))> = None;
        let mut try_fit = |fr: &Rect, rw: u32, rh: u32, rot: bool| {
            if rw <= fr.w && rh <= fr.h {
                let (dw, dh) = (fr.w - rw, fr.h - rh);
                let score = (dw.min(dh), dw.max(dh));
                // 회전은 점수가 확실히 나을 때만(같으면 원래 방향).
                if best.map_or(true, |b| score < b.3) {
                    best = Some((fr.x, fr.y, rot, score));
                }
            }
        };
        for fr in &self.free {
            try_fit(fr, w, h, false);
        }
        if allow_rotate && w != h {
            for fr in &self.free {
                try_fit(fr, h, w, true);
            }
        }
        best
    }

    fn place(&mut self, r: Rect) {
        let mut next = Vec::with_capacity(self.free.len() + 4);
        for fr in &self.free {
            if !fr.intersects(&r) {
                next.push(*fr);
                continue;
            }
            // 겹치는 빈 사각형을 놓인 사각형 바깥의 네 조각(최대)으로 나눈다.
            if r.x > fr.x {
                next.push(Rect { x: fr.x, y: fr.y, w: r.x - fr.x, h: fr.h });
            }
            if r.x + r.w < fr.x + fr.w {
                next.push(Rect { x: r.x + r.w, y: fr.y, w: fr.x + fr.w - (r.x + r.w), h: fr.h });
            }
            if r.y > fr.y {
                next.push(Rect { x: fr.x, y: fr.y, w: fr.w, h: r.y - fr.y });
            }
            if r.y + r.h < fr.y + fr.h {
                next.push(Rect { x: fr.x, y: r.y + r.h, w: fr.w, h: fr.y + fr.h - (r.y + r.h) });
            }
        }
        // 다른 빈 사각형 안에 완전히 들어가는 것은 버린다.
        let mut kept: Vec<Rect> = Vec::with_capacity(next.len());
        for (i, a) in next.iter().enumerate() {
            let covered = next.iter().enumerate().any(|(j, b)| i != j && b.contains(a) && (a != b || j < i));
            if !covered {
                kept.push(*a);
            }
        }
        self.free = kept;
    }
}

/// 배치 결과 하나.
struct Placed {
    input: usize,
    trim: Trim,
    page: usize,
    x: u32,
    y: u32,
    rotated: bool,
}

/// 영역이 서로 겹치지 않고 페이지 안에 들어가게 배치한다. 한 이미지가 최대 크기보다 크면 오류.
pub fn pack_images(inputs: &[PackInput], opts: &PackOptions) -> Result<PackResult> {
    if opts.max_width == 0 || opts.max_height == 0 {
        return Err(CoreError::InvalidArgs("페이지 최대 크기는 0보다 커야 합니다".into()));
    }
    for inp in inputs {
        let img = &inp.image;
        if img.width == 0 || img.height == 0 || img.rgba.len() != (img.width * img.height * 4) as usize {
            return Err(CoreError::InvalidArgs(format!("이미지 '{}'의 크기가 잘못되었습니다", inp.name)));
        }
    }
    let trims: Vec<Trim> = inputs
        .iter()
        .map(|i| if opts.strip_whitespace { trim_rect(&i.image) } else { Trim { x: 0, y: 0, w: i.image.width, h: i.image.height } })
        .collect();
    for (inp, t) in inputs.iter().zip(&trims) {
        let fits = (t.w <= opts.max_width && t.h <= opts.max_height) || (opts.rotation && t.h <= opts.max_width && t.w <= opts.max_height);
        if !fits {
            return Err(CoreError::InvalidArgs(format!(
                "이미지 '{}'({}×{})가 페이지 최대 크기({}×{})보다 큽니다",
                inp.name, t.w, t.h, opts.max_width, opts.max_height
            )));
        }
    }
    // 긴 변 → 넓이 → 이름 순(같은 입력이면 항상 같은 배치).
    let mut order: Vec<usize> = (0..inputs.len()).collect();
    order.sort_by(|&a, &b| {
        let (ta, tb) = (trims[a], trims[b]);
        (tb.w.max(tb.h), tb.w * tb.h).cmp(&(ta.w.max(ta.h), ta.w * ta.h)).then_with(|| inputs[a].name.cmp(&inputs[b].name))
    });
    // 여백: 영역마다 오른쪽·아래에 padding을 붙여 배치하고, 판도 그만큼 넓힌다.
    // 그러면 이웃 영역 사이는 padding 이상 떨어지고, 마지막 영역의 끝은 최대 크기를 넘지 않는다.
    let pad = opts.padding;
    let mut bins: Vec<MaxRects> = Vec::new();
    let mut placed = Vec::with_capacity(inputs.len());
    for &i in &order {
        let t = trims[i];
        let (w, h) = (t.w + pad, t.h + pad);
        let mut spot = None;
        for (pi, bin) in bins.iter().enumerate() {
            if let Some((x, y, rot, _)) = bin.find(w, h, opts.rotation) {
                spot = Some((pi, x, y, rot));
                break;
            }
        }
        let (pi, x, y, rot) = match spot {
            Some(s) => s,
            None => {
                let bin = MaxRects::new(opts.max_width + pad, opts.max_height + pad);
                let (x, y, rot, _) = bin.find(w, h, opts.rotation).ok_or_else(|| CoreError::InvalidArgs(format!("이미지 '{}'를 배치할 수 없습니다", inputs[i].name)))?;
                bins.push(bin);
                (bins.len() - 1, x, y, rot)
            }
        };
        let (pw, ph) = if rot { (h, w) } else { (w, h) };
        let bin = &mut bins[pi];
        bin.place(Rect { x, y, w: pw, h: ph });
        let (sw, sh) = if rot { (t.h, t.w) } else { (t.w, t.h) };
        bin.used_w = bin.used_w.max(x + sw);
        bin.used_h = bin.used_h.max(y + sh);
        placed.push(Placed { input: i, trim: t, page: pi, x, y, rotated: rot });
    }
    // 페이지 합성.
    let mut atlas = Atlas::default();
    let mut pages = Vec::with_capacity(bins.len());
    for (pi, bin) in bins.iter().enumerate() {
        let (mut pw, mut ph) = (bin.used_w.max(1), bin.used_h.max(1));
        if opts.power_of_two {
            pw = pw.next_power_of_two();
            ph = ph.next_power_of_two();
        }
        let file_name = if pi == 0 { format!("{}.png", opts.page_name) } else { format!("{}_{}.png", opts.page_name, pi + 1) };
        let mut image = Image { width: pw, height: ph, rgba: vec![0; (pw * ph * 4) as usize] };
        let mut regions = Vec::new();
        for p in placed.iter().filter(|p| p.page == pi) {
            let src = &inputs[p.input].image;
            blit(src, p.trim, &mut image, p.x, p.y, p.rotated);
            let mut r = AtlasRegion::new(inputs[p.input].name.clone(), p.x, p.y, p.trim.w, p.trim.h);
            r.offset_x = p.trim.x;
            // 아래 공백 = 원래 높이 − (위 공백 + 잘라 쓴 높이).
            r.offset_y = src.height - (p.trim.y + p.trim.h);
            r.original_width = src.width;
            r.original_height = src.height;
            r.degrees = if p.rotated { 90 } else { 0 };
            regions.push(r);
        }
        // atlas에는 입력 순서대로 적는다.
        regions.sort_by_key(|r| inputs.iter().position(|i| i.name == r.name).unwrap_or(usize::MAX));
        if opts.pma {
            premultiply(&mut image);
        }
        atlas.pages.push(AtlasPage {
            name: file_name.clone(),
            width: pw,
            height: ph,
            format: Some("RGBA8888".into()),
            filter: Some(("Linear".into(), "Linear".into())),
            repeat: None,
            pma: opts.pma,
            extra: vec![],
            regions,
        });
        pages.push(PackedPage { file_name, image });
    }
    Ok(PackResult { atlas, pages })
}

/// 잘라낸 부분을 페이지 (x, y)에 복사한다. 회전이면 반시계 90도:
/// 원본 (sx, sy) → 페이지 (x + sy, y + (w − 1 − sx)) (원본 위쪽 줄이 왼쪽 열이 되고, 원본 오른쪽 끝이 위로 간다).
fn blit(src: &Image, t: Trim, dst: &mut Image, x: u32, y: u32, rotated: bool) {
    for sy in 0..t.h {
        for sx in 0..t.w {
            let si = (((t.y + sy) * src.width + t.x + sx) * 4) as usize;
            let (dx, dy) = if rotated { (x + sy, y + (t.w - 1 - sx)) } else { (x + sx, y + sy) };
            let di = ((dy * dst.width + dx) * 4) as usize;
            dst.rgba[di..di + 4].copy_from_slice(&src.rgba[si..si + 4]);
        }
    }
}

/// 색에 알파를 미리 곱한다(반올림).
fn premultiply(img: &mut Image) {
    for p in img.rgba.chunks_exact_mut(4) {
        let a = p[3] as u32;
        for c in &mut p[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 크기 w×h, 테두리 `margin`픽셀은 투명, 안쪽은 위치마다 다른 불투명 색.
    fn img(w: u32, h: u32, margin: u32, seed: u8) -> Image {
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in margin..h - margin {
            for x in margin..w - margin {
                let i = ((y * w + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[seed, x as u8, y as u8, 255]);
            }
        }
        Image { width: w, height: h, rgba }
    }

    fn px(img: &Image, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.width + x) * 4) as usize;
        [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2], img.rgba[i + 3]]
    }

    /// 영역 겹침 없음, 페이지 안, 페이지 픽셀 = 원본(회전·잘라내기 반영).
    fn check(inputs: &[PackInput], opts: &PackOptions, r: &PackResult) {
        let total: usize = r.atlas.pages.iter().map(|p| p.regions.len()).sum();
        assert_eq!(total, inputs.len());
        for (page, packed) in r.atlas.pages.iter().zip(&r.pages) {
            assert_eq!((page.width, page.height), (packed.image.width, packed.image.height));
            assert_eq!(page.name, packed.file_name);
            assert_eq!(page.filter, Some(("Linear".into(), "Linear".into())));
            let boxes: Vec<(u32, u32, u32, u32)> = page
                .regions
                .iter()
                .map(|g| if g.degrees == 90 { (g.x, g.y, g.height, g.width) } else { (g.x, g.y, g.width, g.height) })
                .collect();
            for (i, a) in boxes.iter().enumerate() {
                assert!(a.0 + a.2 <= page.width && a.1 + a.3 <= page.height, "페이지 밖: {a:?}");
                assert!(a.0 + a.2 <= opts.max_width && a.1 + a.3 <= opts.max_height);
                for b in &boxes[i + 1..] {
                    // 여백까지 포함해 떨어져 있어야 한다.
                    let p = opts.padding;
                    let apart = a.0 + a.2 + p <= b.0 || b.0 + b.2 + p <= a.0 || a.1 + a.3 + p <= b.1 || b.1 + b.3 + p <= a.1;
                    assert!(apart, "겹침: {a:?} {b:?}");
                }
            }
            for g in &page.regions {
                let src = &inputs.iter().find(|i| i.name == g.name).unwrap().image;
                assert_eq!((g.original_width, g.original_height), (src.width, src.height));
                // 원본에서 잘라낸 위 공백 = 원래 높이 − 아래 공백 − 높이.
                let top = g.original_height - g.offset_y - g.height;
                for sy in 0..g.height {
                    for sx in 0..g.width {
                        let (dx, dy) = if g.degrees == 90 { (g.x + sy, g.y + g.width - 1 - sx) } else { (g.x + sx, g.y + sy) };
                        assert_eq!(px(&packed.image, dx, dy), px(src, g.offset_x + sx, top + sy), "{} ({sx},{sy})", g.name);
                    }
                }
            }
        }
    }

    fn inputs() -> Vec<PackInput> {
        let sizes = [(40, 30, 3), (10, 64, 0), (64, 10, 2), (25, 25, 5), (8, 8, 0), (50, 12, 1), (12, 50, 1), (33, 7, 0)];
        sizes.iter().enumerate().map(|(i, &(w, h, m))| PackInput { name: format!("img{i}"), image: img(w, h, m, i as u8 * 20) }).collect()
    }

    #[test]
    fn k01_pack_no_overlap_and_pixels_match() {
        let inp = inputs();
        let opts = PackOptions { max_width: 128, max_height: 128, ..PackOptions::default() };
        let r = pack_images(&inp, &opts).unwrap();
        assert_eq!(r.pages.len(), 1);
        assert_eq!(r.pages[0].file_name, "skeleton.png");
        check(&inp, &opts, &r);
        let page = &r.atlas.pages[0];
        assert!(page.width.is_power_of_two() && page.height.is_power_of_two());
        // img0: 40×30, 테두리 3 투명 → 34×24, offsets 왼쪽 3, 아래 3, 원래 40×30.
        let g = page.regions.iter().find(|g| g.name == "img0").unwrap();
        assert_eq!((g.width, g.height, g.offset_x, g.offset_y, g.original_width, g.original_height), (34, 24, 3, 3, 40, 30));
        // atlas에는 입력 순서대로.
        let names: Vec<_> = page.regions.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["img0", "img1", "img2", "img3", "img4", "img5", "img6", "img7"]);
    }

    #[test]
    fn k01_pack_offsets_bottom_left() {
        // 10×10에서 불투명 칸이 x 2..5, y 1..4(위에서부터)인 이미지.
        // 잘라낸 크기 3×3, 왼쪽 공백 2, 아래 공백 = 10 − 4 = 6.
        let mut im = Image { width: 10, height: 10, rgba: vec![0; 400] };
        for y in 1..4 {
            for x in 2..5 {
                im.rgba[(y * 10 + x) * 4 + 3] = 255;
            }
        }
        let r = pack_images(&[PackInput { name: "a".into(), image: im }], &PackOptions::default()).unwrap();
        let g = &r.atlas.pages[0].regions[0];
        assert_eq!((g.width, g.height, g.offset_x, g.offset_y), (3, 3, 2, 6));
        // 2의 거듭제곱: 3 → 4.
        assert_eq!((r.atlas.pages[0].width, r.atlas.pages[0].height), (4, 4));
    }

    #[test]
    fn k01_pack_rotation_and_multi_page() {
        // 128×16 판에 16×60 두 장: 회전 없이는 높이 60 > 16이라 못 들어가고, 회전하면 60×16 두 장이 옆으로 들어간다(120 ≤ 128).
        let inp: Vec<PackInput> = (0..2).map(|i| PackInput { name: format!("t{i}"), image: img(16, 60, 0, 50 + i) }).collect();
        let opts = PackOptions { max_width: 128, max_height: 16, padding: 0, rotation: true, power_of_two: false, ..PackOptions::default() };
        let r = pack_images(&inp, &opts).unwrap();
        check(&inp, &opts, &r);
        assert_eq!(r.pages.len(), 1);
        assert!(r.atlas.pages[0].regions.iter().all(|g| g.degrees == 90 && g.width == 16 && g.height == 60));
        // 회전 금지면 이 판에는 들어갈 수 없어 오류.
        let no_rot = PackOptions { rotation: false, ..opts.clone() };
        assert!(matches!(pack_images(&inp, &no_rot), Err(CoreError::InvalidArgs(_))));

        // 64×64 판에 50×50 세 장 → 페이지 3장, 이름 page.png, page_2.png, page_3.png.
        let inp: Vec<PackInput> = (0..3).map(|i| PackInput { name: format!("b{i}"), image: img(50, 50, 0, i) }).collect();
        let opts = PackOptions { max_width: 64, max_height: 64, page_name: "page".into(), ..PackOptions::default() };
        let r = pack_images(&inp, &opts).unwrap();
        check(&inp, &opts, &r);
        let names: Vec<_> = r.pages.iter().map(|p| p.file_name.as_str()).collect();
        assert_eq!(names, ["page.png", "page_2.png", "page_3.png"]);
    }

    #[test]
    fn k01_pack_too_large_is_error() {
        let inp = [PackInput { name: "big".into(), image: img(300, 20, 0, 1) }];
        let opts = PackOptions { max_width: 256, max_height: 256, ..PackOptions::default() };
        assert!(matches!(pack_images(&inp, &opts), Err(CoreError::InvalidArgs(_))));
        // 공백을 잘라내면 들어가는 경우는 된다(300×20 중 가운데 200×10만 불투명).
        let mut im = Image { width: 300, height: 20, rgba: vec![0; 300 * 20 * 4] };
        for y in 5..15 {
            for x in 50..250 {
                im.rgba[(y * 300 + x) * 4 + 3] = 255;
            }
        }
        assert!(pack_images(&[PackInput { name: "ok".into(), image: im }], &opts).is_ok());
    }

    #[test]
    fn k01_pack_pma_and_empty() {
        // 알파 128, 색 200 → 200×128/255 = 100.4 → 100.
        let im = Image { width: 1, height: 1, rgba: vec![200, 0, 255, 128] };
        let opts = PackOptions { pma: true, ..PackOptions::default() };
        let r = pack_images(&[PackInput { name: "p".into(), image: im }], &opts).unwrap();
        assert!(r.atlas.pages[0].pma);
        assert_eq!(&r.pages[0].image.rgba[..4], &[100, 0, 128, 128]);
        // 입력이 없으면 페이지도 없다.
        let r = pack_images(&[], &PackOptions::default()).unwrap();
        assert!(r.pages.is_empty() && r.atlas.pages.is_empty());
    }
}
