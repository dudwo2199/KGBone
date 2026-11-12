//! .atlas 텍스트 → Atlas. 4.x 표기(bounds, offsets)와 이전 표기(xy, size, orig, offset)를 모두 읽는다.
//!
//! - 빈 줄 다음의 첫 줄(또는 파일 첫 줄)은 페이지 이름, 그 뒤 `키:값` 줄은 페이지 필드.
//! - 페이지 안에서 `:`가 없는 줄은 영역 이름, 그 뒤 `키:값` 줄은 영역 필드.
//! - 회전 저장(90·270도)된 영역의 bounds 폭·높이는 페이지에 놓인 그대로이므로, 원래 방향으로 바꿔 보관한다.
use super::{Atlas, AtlasPage, AtlasRegion};
use crate::{CoreError, Result};

pub fn parse(text: &str) -> Result<Atlas> {
    let mut atlas = Atlas::default();
    let mut new_page = true;
    // 지금 읽는 영역에서 원래 크기·공백을 따로 받았는지(이전 표기 orig/offset 또는 offsets).
    let mut region_open = false;
    let mut orig_given = false;
    for (n, line) in text.lines().enumerate() {
        let err = |m: &str| CoreError::Parse(format!("atlas {}번째 줄: {m}", n + 1));
        let t = line.trim();
        if t.is_empty() {
            if region_open {
                finish(&mut atlas, orig_given);
                region_open = false;
            }
            new_page = true;
            continue;
        }
        match t.split_once(':') {
            None => {
                if region_open {
                    finish(&mut atlas, orig_given);
                    region_open = false;
                }
                if new_page {
                    atlas.pages.push(AtlasPage {
                        name: t.to_string(),
                        width: 0,
                        height: 0,
                        format: None,
                        filter: None,
                        repeat: None,
                        pma: false,
                        extra: vec![],
                        regions: vec![],
                    });
                    new_page = false;
                } else {
                    let page = atlas.pages.last_mut().ok_or_else(|| err("페이지 없이 영역이 나왔습니다"))?;
                    page.regions.push(AtlasRegion::new(t, 0, 0, 0, 0));
                    region_open = true;
                    orig_given = false;
                }
            }
            Some((k, v)) => {
                let (k, v) = (k.trim(), v.trim());
                let page = atlas.pages.last_mut().ok_or_else(|| err("페이지 이름이 먼저 와야 합니다"))?;
                new_page = false;
                if region_open {
                    let r = page.regions.last_mut().expect("영역");
                    region_field(r, k, v, &mut orig_given).map_err(|m| err(&m))?;
                } else {
                    page_field(page, k, v).map_err(|m| err(&m))?;
                }
            }
        }
    }
    if region_open {
        finish(&mut atlas, orig_given);
    }
    Ok(atlas)
}

fn nums(v: &str, n: usize) -> std::result::Result<Vec<i64>, String> {
    let out: Vec<i64> = v.split(',').map(|x| x.trim().parse::<i64>()).collect::<std::result::Result<_, _>>().map_err(|_| format!("숫자 형식이 잘못됨: {v}"))?;
    if out.len() < n {
        return Err(format!("숫자 {n}개가 필요합니다: {v}"));
    }
    Ok(out)
}

fn u(x: i64) -> u32 {
    x.max(0) as u32
}

fn page_field(p: &mut AtlasPage, k: &str, v: &str) -> std::result::Result<(), String> {
    match k {
        "size" => {
            let a = nums(v, 2)?;
            p.width = u(a[0]);
            p.height = u(a[1]);
        }
        "format" => p.format = Some(v.to_string()),
        "filter" => {
            let (a, b) = v.split_once(',').ok_or_else(|| format!("filter 값이 잘못됨: {v}"))?;
            p.filter = Some((a.trim().to_string(), b.trim().to_string()));
        }
        "repeat" => p.repeat = Some(v.to_string()),
        "pma" => p.pma = v == "true",
        _ => p.extra.push((k.to_string(), v.to_string())),
    }
    Ok(())
}

fn region_field(r: &mut AtlasRegion, k: &str, v: &str, orig_given: &mut bool) -> std::result::Result<(), String> {
    match k {
        "bounds" => {
            let a = nums(v, 4)?;
            (r.x, r.y, r.width, r.height) = (u(a[0]), u(a[1]), u(a[2]), u(a[3]));
        }
        "xy" => {
            let a = nums(v, 2)?;
            (r.x, r.y) = (u(a[0]), u(a[1]));
        }
        "size" => {
            let a = nums(v, 2)?;
            (r.width, r.height) = (u(a[0]), u(a[1]));
        }
        "offsets" => {
            let a = nums(v, 4)?;
            (r.offset_x, r.offset_y, r.original_width, r.original_height) = (u(a[0]), u(a[1]), u(a[2]), u(a[3]));
            *orig_given = true;
        }
        "orig" => {
            let a = nums(v, 2)?;
            (r.original_width, r.original_height) = (u(a[0]), u(a[1]));
            *orig_given = true;
        }
        "offset" => {
            let a = nums(v, 2)?;
            (r.offset_x, r.offset_y) = (u(a[0]), u(a[1]));
        }
        "rotate" => {
            r.degrees = match v {
                "true" => 90,
                "false" => 0,
                d => d.parse::<i64>().map_err(|_| format!("rotate 값이 잘못됨: {v}"))?.rem_euclid(360) as u32,
            }
        }
        "index" => r.index = v.parse().map_err(|_| format!("index 값이 잘못됨: {v}"))?,
        "split" => {
            let a = nums(v, 4)?;
            r.split = Some([a[0] as i32, a[1] as i32, a[2] as i32, a[3] as i32]);
        }
        "pad" => {
            let a = nums(v, 4)?;
            r.pad = Some([a[0] as i32, a[1] as i32, a[2] as i32, a[3] as i32]);
        }
        _ => r.extra.push((k.to_string(), v.to_string())),
    }
    Ok(())
}

/// 영역 하나를 다 읽은 뒤: 회전 저장이면 폭·높이를 원래 방향으로, 원래 크기가 없으면 폭·높이로.
fn finish(atlas: &mut Atlas, orig_given: bool) {
    let Some(r) = atlas.pages.last_mut().and_then(|p| p.regions.last_mut()) else { return };
    if r.degrees == 90 || r.degrees == 270 {
        std::mem::swap(&mut r.width, &mut r.height);
    }
    if !orig_given {
        r.original_width = r.width;
        r.original_height = r.height;
    }
}
