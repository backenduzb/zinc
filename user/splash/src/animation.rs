use crate::graphics::{
    COLOR_ACCENT,
    COLOR_BG,
    COLOR_PANEL,
    clear,
    draw_logo_faded,
    logo_dimensions,
    rect,
};
use crate::syscall::{uptime_ms, yield_now};

const BAR_MAX_WIDTH: u32 = 240;
const BAR_HEIGHT: u32 = 4;
const BAR_GAP: u32 = 18;
const LOGO_FADE_MS: u64 = 800;
const LOADING_DURATION_MS: u64 = 2400;
const COLOR_OK: u32 = 0x00FFFFFF;
const GLYPH_W: u32 = 5;
const GLYPH_H: u32 = 7;
const GLYPH_SCALE: u32 = 3;
const GLYPH_GAP: u32 = 2;
const OK_W: u32 = (GLYPH_W + GLYPH_GAP + GLYPH_W) * GLYPH_SCALE;
const OK_H: u32 = GLYPH_H * GLYPH_SCALE;
const O_GLYPH: [u8; 7] = [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110];
const K_GLYPH: [u8; 7] = [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001];

fn wait_until(deadline: u64) {
    while uptime_ms() < deadline {
        yield_now();
    }
}

fn rounded_bar(x: u32, y: u32, width: u32, height: u32, color: u32) -> bool {
    if width == 0 || height == 0 {
        return true;
    }

    let radius = (height / 2).min(width / 2);

    for row in 0..height {
        let distance = row.min(height - 1 - row);
        let inset = radius.saturating_sub(distance).min(width / 2);
        let row_width = width.saturating_sub(inset * 2);

        if row_width == 0 {
            continue;
        }

        if !rect(x + inset, y + row, row_width, 1, color) {
            return false;
        }
    }

    true
}

fn draw_progress_growth(x: u32, y: u32, old_width: u32, new_width: u32, color: u32) -> bool {
    if new_width <= old_width {
        return true;
    }

    if old_width < BAR_HEIGHT {
        return rounded_bar(x, y, new_width, BAR_HEIGHT, color);
    }

    let radius = BAR_HEIGHT / 2;
    let mut row = 0;

    while row < BAR_HEIGHT {
        let distance = row.min(BAR_HEIGHT - 1 - row);
        let inset = radius.saturating_sub(distance).min(new_width / 2);
        let old_row_width = old_width.saturating_sub(inset * 2);
        let new_row_width = new_width.saturating_sub(inset * 2);

        if new_row_width > old_row_width
            && !rect(
                x + inset + old_row_width,
                y + row,
                new_row_width - old_row_width,
                1,
                color,
            )
        {
            return false;
        }

        row += 1;
    }

    true
}

fn eased_progress(elapsed: u64, bar_width: u32) -> u32 {
    if elapsed >= LOADING_DURATION_MS {
        return bar_width;
    }

    let denom = LOADING_DURATION_MS * LOADING_DURATION_MS;
    let inv = LOADING_DURATION_MS - elapsed;
    let numer = denom - inv * inv;

    ((bar_width as u64 * numer / denom) as u32).min(bar_width)
}

fn draw_glyph(x: u32, y: u32, glyph: &[u8; 7], color: u32) -> bool {
    let mut row: u32 = 0;

    while row < GLYPH_H {
        let bits = glyph[row as usize];
        let mut col: u32 = 0;

        while col < GLYPH_W {
            if ((bits >> (GLYPH_W - 1 - col)) & 1) == 1 {
                let start = col;

                while col < GLYPH_W && ((bits >> (GLYPH_W - 1 - col)) & 1) == 1 {
                    col += 1;
                }

                if !rect(
                    x + start * GLYPH_SCALE,
                    y + row * GLYPH_SCALE,
                    (col - start) * GLYPH_SCALE,
                    GLYPH_SCALE,
                    color,
                ) {
                    return false;
                }
            } else {
                col += 1;
            }
        }

        row += 1;
    }

    true
}

fn draw_ok(screen_width: u32, screen_height: u32) -> bool {
    let x0 = screen_width.saturating_sub(OK_W) / 2;
    let y0 = screen_height.saturating_sub(OK_H) / 2;

    if !draw_glyph(x0, y0, &O_GLYPH, COLOR_OK) {
        return false;
    }

    if !draw_glyph(x0 + (GLYPH_W + GLYPH_GAP) * GLYPH_SCALE, y0, &K_GLYPH, COLOR_OK) {
        return false;
    }

    true
}

pub fn run(screen_width: u32, screen_height: u32) -> bool {
    let (logo_width, logo_height) = match logo_dimensions() {
        Some(dimensions) => dimensions,
        None => return false,
    };

    let bar_width = BAR_MAX_WIDTH.min(screen_width.saturating_sub(32));

    if bar_width < BAR_HEIGHT
        || logo_width > screen_width
        || logo_height.saturating_add(BAR_GAP + BAR_HEIGHT) > screen_height
    {
        return false;
    }

    if !clear(COLOR_BG) {
        return false;
    }

    let content_height = logo_height + BAR_GAP + BAR_HEIGHT;
    let logo_y = screen_height.saturating_sub(content_height) / 2;
    let bar_x = (screen_width - bar_width) / 2;
    let bar_y = logo_y + logo_height + BAR_GAP;

    let start = uptime_ms();
    let mut last_alpha = 0u8;
    let mut have_frame = false;
    let mut frame: u64 = 0;

    loop {
        let elapsed = uptime_ms().saturating_sub(start).min(LOGO_FADE_MS);
        let alpha = (elapsed * 255 / LOGO_FADE_MS) as u8;

        if !have_frame || alpha != last_alpha {
            if !draw_logo_faded(screen_width, screen_height, alpha) {
                return false;
            }

            last_alpha = alpha;
            have_frame = true;
        }

        if elapsed >= LOGO_FADE_MS {
            break;
        }

        frame += 1;
        wait_until(start + frame * 1000 / 60);
    }

    if !rounded_bar(bar_x, bar_y, bar_width, BAR_HEIGHT, COLOR_PANEL) {
        return false;
    }

    let loading_start = uptime_ms();
    let mut last_progress: u32 = 0;
    let mut loading_frame: u64 = 0;

    loop {
        let elapsed = uptime_ms()
            .saturating_sub(loading_start)
            .min(LOADING_DURATION_MS);
        let progress = eased_progress(elapsed, bar_width);

        if progress != last_progress {
            if !draw_progress_growth(bar_x, bar_y, last_progress, progress, COLOR_ACCENT) {
                return false;
            }

            last_progress = progress;
        }

        if elapsed >= LOADING_DURATION_MS {
            break;
        }

        loading_frame += 1;
        wait_until(loading_start + loading_frame * 1000 / 60);
    }

    if !clear(COLOR_BG) {
        return false;
    }

    if !draw_ok(screen_width, screen_height) {
        return false;
    }

    yield_now();
    true
}
