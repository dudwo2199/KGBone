//! Atlas → .atlas 텍스트(4.x 표기). 기본값 필드는 생략한다(offsets: 공백 없으면, rotate: 0이면, index: -1이면).
use super::Atlas;

pub fn write(atlas: &Atlas) -> String {
    let mut out = String::new();
    for (i, p) in atlas.pages.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("{}\nsize:{},{}\n", p.name, p.width, p.height));
        if let Some(f) = &p.format {
            out.push_str(&format!("format:{f}\n"));
        }
        if let Some((a, b)) = &p.filter {
            out.push_str(&format!("filter:{a},{b}\n"));
        }
        if let Some(r) = &p.repeat {
            out.push_str(&format!("repeat:{r}\n"));
        }
        if p.pma {
            out.push_str("pma:true\n");
        }
        for (k, v) in &p.extra {
            out.push_str(&format!("{k}:{v}\n"));
        }
        for r in &p.regions {
            // 회전 저장이면 페이지에 놓인 그대로의 폭·높이로 쓴다.
            let (w, h) = if r.degrees == 90 || r.degrees == 270 { (r.height, r.width) } else { (r.width, r.height) };
            out.push_str(&format!("{}\nbounds:{},{},{},{}\n", r.name, r.x, r.y, w, h));
            if r.offset_x != 0 || r.offset_y != 0 || r.original_width != r.width || r.original_height != r.height {
                out.push_str(&format!("offsets:{},{},{},{}\n", r.offset_x, r.offset_y, r.original_width, r.original_height));
            }
            if r.degrees != 0 {
                out.push_str(&format!("rotate:{}\n", r.degrees));
            }
            if r.index != -1 {
                out.push_str(&format!("index:{}\n", r.index));
            }
            if let Some(s) = r.split {
                out.push_str(&format!("split:{},{},{},{}\n", s[0], s[1], s[2], s[3]));
            }
            if let Some(s) = r.pad {
                out.push_str(&format!("pad:{},{},{},{}\n", s[0], s[1], s[2], s[3]));
            }
            for (k, v) in &r.extra {
                out.push_str(&format!("{k}:{v}\n"));
            }
        }
    }
    out
}
