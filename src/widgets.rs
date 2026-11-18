use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};
use std::f32::consts::TAU;

/// Helper to create a properly premultiplied translucent `Color32`.
#[inline]
pub fn rgba_premul(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    let af = a as f32 / 255.0;
    Color32::from_rgba_premultiplied(
        (r as f32 * af).round() as u8,
        (g as f32 * af).round() as u8,
        (b as f32 * af).round() as u8,
        a,
    )
}

/// Paints the dark modern background with a subtle technical dot/grid mesh.
pub fn paint_aurora_background(ui: &Ui, rect: Rect) {
    let painter = ui.painter();

    // Deep midnight slate base
    painter.rect_filled(rect, 0.0, Color32::from_rgb(13, 18, 30));

    // Subtle technical grid lines across the background
    let step = 32.0;
    let line_stroke = Stroke::new(1.0_f32, rgba_premul(56, 189, 248, 8));
    let mut x = rect.left() + step;
    while x < rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            line_stroke,
        );
        x += step;
    }
    let mut y = rect.top() + step;
    while y < rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            line_stroke,
        );
        y += step;
    }
}

/// Draws the glowing emerald lightning bolt icon in the main header.
pub fn draw_header_lightning(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 30.0), Sense::hover());
    let painter = ui.painter();
    let c = rect.center();

    let top_half = vec![
        Pos2::new(c.x + 3.5, c.y - 12.5),
        Pos2::new(c.x - 8.5, c.y + 1.2),
        Pos2::new(c.x + 1.0, c.y + 1.2),
    ];
    let bottom_half = vec![
        Pos2::new(c.x - 1.0, c.y - 1.2),
        Pos2::new(c.x + 8.5, c.y - 1.2),
        Pos2::new(c.x - 3.5, c.y + 12.5),
    ];

    let bolt_color = Color32::from_rgb(52, 211, 153);
    painter.add(Shape::convex_polygon(top_half, bolt_color, Stroke::NONE));
    painter.add(Shape::convex_polygon(bottom_half, bolt_color, Stroke::NONE));
}

/// Draws a glowing status pill badge (e.g., "ONLINE" with neon emerald border).
pub fn draw_glowing_status_pill(ui: &mut Ui, label: &str, fill: Color32, text_color: Color32) {
    let font = FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_color);
    let pill_size = Vec2::new(galley.size().x + 28.0, 30.0);
    let (rect, _) = ui.allocate_exact_size(pill_size, Sense::hover());
    let painter = ui.painter();

    painter.rect_stroke(
        rect.expand(2.0),
        Rounding::same(16.0),
        Stroke::new(2.0_f32, rgba_premul(fill.r(), fill.g(), fill.b(), 65)),
    );

    painter.rect_filled(rect, Rounding::same(15.0), fill);
    painter.rect_stroke(
        rect,
        Rounding::same(15.0),
        Stroke::new(1.0_f32, rgba_premul(255, 255, 255, 90)),
    );

    painter.galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        text_color,
    );
}

/// Draws a glowing battery progress bar with inner text and threshold indicator.
pub fn draw_glowing_battery_bar(
    ui: &mut Ui,
    charge_percent: u8,
    threshold_percent: u8,
    on_battery: bool,
) {
    let height = 24.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, Rounding::same(12.0), Color32::from_rgb(12, 18, 30));

    let frac = (charge_percent as f32 / 100.0).clamp(0.0, 1.0);
    let (bar_color, text_color) = if charge_percent <= threshold_percent {
        (Color32::from_rgb(239, 68, 68), Color32::WHITE)
    } else if on_battery {
        (
            Color32::from_rgb(245, 158, 11),
            Color32::from_rgb(15, 23, 42),
        )
    } else {
        (
            Color32::from_rgb(52, 211, 153),
            Color32::from_rgb(6, 46, 34),
        )
    };

    if frac > 0.0 {
        let fill_rect = Rect::from_min_max(
            rect.min,
            Pos2::new(rect.left() + rect.width() * frac, rect.bottom()),
        );
        painter.rect_filled(fill_rect, Rounding::same(12.0), bar_color);
    }

    painter.rect_stroke(
        rect.expand(1.0),
        Rounding::same(13.0),
        Stroke::new(
            1.5_f32,
            rgba_premul(bar_color.r(), bar_color.g(), bar_color.b(), 55),
        ),
    );

    painter.text(
        Pos2::new(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        format!(
            "Battery Charge: {}% (Trigger Threshold: {}%)",
            charge_percent, threshold_percent
        ),
        FontId::proportional(12.8),
        text_color,
    );
}

/// Custom vector line icons used in telemetry, action cards, and footer buttons.
#[derive(Debug, Clone, Copy)]
pub enum VectorIcon {
    LoadWeight,
    AcOutlet,
    BatteryMini,
    Bell,
    Speaker,
    PowerOff,
    Terminal,
    FloppySave,
    PlayTriangle,
}

/// Paints a crisp vector line icon at `rect`.
pub fn paint_vector_icon(painter: &egui::Painter, rect: Rect, icon: VectorIcon, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.5_f32, color);

    match icon {
        VectorIcon::LoadWeight => {
            let body = vec![
                Pos2::new(c.x - 4.5, c.y - 2.0),
                Pos2::new(c.x + 4.5, c.y - 2.0),
                Pos2::new(c.x + 6.0, c.y + 6.0),
                Pos2::new(c.x - 6.0, c.y + 6.0),
            ];
            painter.add(Shape::closed_line(body, stroke));
            painter.circle_stroke(Pos2::new(c.x, c.y - 3.5), 2.3, stroke);
        }
        VectorIcon::AcOutlet => {
            let box_rect = Rect::from_center_size(c, Vec2::new(13.0, 13.0));
            painter.rect_stroke(box_rect, Rounding::same(2.5), stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.5, c.y - 3.2),
                    Pos2::new(c.x - 2.5, c.y - 0.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 2.5, c.y - 3.2),
                    Pos2::new(c.x + 2.5, c.y - 0.5),
                ],
                stroke,
            );
            painter.circle_filled(Pos2::new(c.x, c.y + 2.8), 1.3, color);
        }
        VectorIcon::BatteryMini => {
            let body = Rect::from_center_size(Pos2::new(c.x - 0.8, c.y), Vec2::new(13.0, 8.5));
            painter.rect_stroke(body, Rounding::same(2.0), stroke);
            let nub = Rect::from_center_size(Pos2::new(c.x + 6.7, c.y), Vec2::new(1.8, 4.0));
            painter.rect_filled(nub, Rounding::same(0.8), color);
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.5, c.y + 1.5),
                    Pos2::new(c.x + 0.5, c.y - 1.5),
                ],
                Stroke::new(1.3_f32, color),
            );
        }
        VectorIcon::Bell => {
            let pts = vec![
                Pos2::new(c.x - 5.5, c.y + 3.5),
                Pos2::new(c.x - 4.0, c.y - 1.5),
                Pos2::new(c.x - 2.2, c.y - 4.8),
                Pos2::new(c.x, c.y - 5.8),
                Pos2::new(c.x + 2.2, c.y - 4.8),
                Pos2::new(c.x + 4.0, c.y - 1.5),
                Pos2::new(c.x + 5.5, c.y + 3.5),
            ];
            painter.add(Shape::closed_line(pts, stroke));
            painter.circle_stroke(Pos2::new(c.x, c.y + 5.2), 1.5, stroke);
        }
        VectorIcon::Speaker => {
            let cone = vec![
                Pos2::new(c.x - 6.0, c.y - 2.2),
                Pos2::new(c.x - 3.0, c.y - 2.2),
                Pos2::new(c.x + 1.0, c.y - 5.5),
                Pos2::new(c.x + 1.0, c.y + 5.5),
                Pos2::new(c.x - 3.0, c.y + 2.2),
                Pos2::new(c.x - 6.0, c.y + 2.2),
            ];
            painter.add(Shape::closed_line(cone, stroke));
            painter.line_segment(
                [Pos2::new(c.x + 3.5, c.y - 2.5), Pos2::new(c.x + 4.5, c.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 4.5, c.y), Pos2::new(c.x + 3.5, c.y + 2.5)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 5.8, c.y - 4.5), Pos2::new(c.x + 7.0, c.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 7.0, c.y), Pos2::new(c.x + 5.8, c.y + 4.5)],
                stroke,
            );
        }
        VectorIcon::PowerOff => {
            let r = 6.0;
            let mut arc_pts = Vec::with_capacity(20);
            for i in 0..=18 {
                let angle = -std::f32::consts::FRAC_PI_2 + 0.65 + (i as f32 / 18.0) * (TAU - 1.3);
                arc_pts.push(Pos2::new(c.x + r * angle.cos(), c.y + r * angle.sin()));
            }
            painter.add(Shape::line(arc_pts, stroke));
            painter.line_segment(
                [Pos2::new(c.x, c.y - 7.0), Pos2::new(c.x, c.y - 0.5)],
                stroke,
            );
        }
        VectorIcon::Terminal => {
            let box_rect = Rect::from_center_size(c, Vec2::new(15.0, 12.0));
            painter.rect_stroke(box_rect, Rounding::same(2.5), stroke);
            painter.line_segment(
                [Pos2::new(c.x - 4.2, c.y - 2.5), Pos2::new(c.x - 1.5, c.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x - 1.5, c.y), Pos2::new(c.x - 4.2, c.y + 2.5)],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 0.5, c.y + 2.5),
                    Pos2::new(c.x + 4.2, c.y + 2.5),
                ],
                stroke,
            );
        }
        VectorIcon::FloppySave => {
            let box_rect = Rect::from_center_size(c, Vec2::new(13.0, 13.0));
            painter.rect_stroke(box_rect, Rounding::same(2.0), stroke);
            let top_shutter =
                Rect::from_center_size(Pos2::new(c.x, c.y - 3.5), Vec2::new(7.0, 4.0));
            painter.rect_stroke(top_shutter, Rounding::same(1.0), stroke);
            let bot_label = Rect::from_center_size(Pos2::new(c.x, c.y + 3.2), Vec2::new(8.5, 5.0));
            painter.rect_stroke(bot_label, Rounding::same(1.0), stroke);
        }
        VectorIcon::PlayTriangle => {
            let tri = vec![
                Pos2::new(c.x - 3.5, c.y - 5.0),
                Pos2::new(c.x + 5.0, c.y),
                Pos2::new(c.x - 3.5, c.y + 5.0),
            ];
            painter.add(Shape::closed_line(tri, stroke));
        }
    }
}

/// Draws a prominent KPI stat card with a vector icon, large bold headline value, and subtitle.
pub fn draw_big_kpi_tile(
    ui: &mut Ui,
    width: f32,
    icon: VectorIcon,
    label: &str,
    big_value: &str,
    sub_text: &str,
    accent_color: Color32,
) {
    let height = 66.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, Rounding::same(9.0), Color32::from_rgb(14, 21, 35));
    painter.rect_stroke(
        rect,
        Rounding::same(9.0),
        Stroke::new(1.0_f32, Color32::from_rgb(36, 51, 76)),
    );

    // Left vertical accent bar
    let bar_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, rect.top() + 10.0),
        Pos2::new(rect.left() + 4.0, rect.bottom() - 10.0),
    );
    painter.rect_filled(bar_rect, Rounding::same(2.0), accent_color);

    // Icon & label row
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 18.0, rect.top() + 14.0),
        Vec2::new(15.0, 15.0),
    );
    paint_vector_icon(&painter, icon_rect, icon, accent_color);

    painter.text(
        Pos2::new(rect.left() + 30.0, rect.top() + 14.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::from_rgb(148, 163, 184),
    );

    // Large bold value
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.top() + 35.0),
        Align2::LEFT_CENTER,
        big_value,
        FontId::proportional(18.5),
        Color32::from_rgb(248, 250, 252),
    );

    // Subtitle on bottom
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.bottom() - 10.0),
        Align2::LEFT_CENTER,
        sub_text,
        FontId::proportional(11.0),
        Color32::from_rgb(120, 138, 165),
    );
}

/// Draws a large, high-visibility telemetry trendline graph with gridlines, Y-axis scale,
/// area fill, glowing curve, and bold live value callout.
#[allow(clippy::too_many_arguments)]
pub fn draw_large_trend_graph(
    ui: &mut Ui,
    width: f32,
    height: f32,
    title: &str,
    big_value: &str,
    sub_value: &str,
    values: &[f32],
    min_val: f32,
    max_val: f32,
    unit_suffix: &str,
    color: Color32,
    threshold_line: Option<f32>,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, Rounding::same(10.0), Color32::from_rgb(13, 20, 34));
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(1.0_f32, Color32::from_rgb(38, 54, 80)),
    );

    let pad = 12.0;

    // Header: Title on left, Big Value + Sub Value on right
    painter.text(
        Pos2::new(rect.left() + pad, rect.top() + 10.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(12.0),
        Color32::from_rgb(165, 180, 202),
    );
    painter.text(
        Pos2::new(rect.left() + pad, rect.top() + 25.0),
        Align2::LEFT_TOP,
        sub_value,
        FontId::proportional(11.0),
        Color32::from_rgb(100, 116, 139),
    );
    painter.text(
        Pos2::new(rect.right() - pad, rect.top() + 8.0),
        Align2::RIGHT_TOP,
        big_value,
        FontId::proportional(19.0),
        color,
    );

    // Plot area
    let plot_rect = Rect::from_min_max(
        Pos2::new(rect.left() + pad + 28.0, rect.top() + 44.0),
        Pos2::new(rect.right() - pad, rect.bottom() - 10.0),
    );

    // Horizontal grid lines + Y-axis labels
    let grid_stroke = Stroke::new(1.0_f32, rgba_premul(71, 85, 105, 70));
    for (frac, val) in [
        (0.0_f32, max_val),
        (0.5_f32, (min_val + max_val) * 0.5),
        (1.0_f32, min_val),
    ] {
        let y = plot_rect.top() + plot_rect.height() * frac;
        painter.line_segment(
            [
                Pos2::new(plot_rect.left(), y),
                Pos2::new(plot_rect.right(), y),
            ],
            grid_stroke,
        );
        painter.text(
            Pos2::new(plot_rect.left() - 5.0, y),
            Align2::RIGHT_CENTER,
            format!("{:.0}{}", val, unit_suffix),
            FontId::proportional(9.5),
            Color32::from_rgb(100, 116, 139),
        );
    }

    // Vertical time grid lines
    for i in 1..4 {
        let x = plot_rect.left() + (i as f32 / 4.0) * plot_rect.width();
        painter.line_segment(
            [
                Pos2::new(x, plot_rect.top()),
                Pos2::new(x, plot_rect.bottom()),
            ],
            Stroke::new(1.0_f32, rgba_premul(51, 65, 85, 45)),
        );
    }

    let span = (max_val - min_val).max(1.0);

    // Optional threshold line (e.g., Battery Alert Threshold)
    if let Some(thresh) = threshold_line {
        let norm = ((thresh - min_val) / span).clamp(0.0, 1.0);
        let ty = plot_rect.bottom() - norm * plot_rect.height();
        painter.line_segment(
            [
                Pos2::new(plot_rect.left(), ty),
                Pos2::new(plot_rect.right(), ty),
            ],
            Stroke::new(1.5_f32, rgba_premul(248, 113, 113, 180)),
        );
        painter.text(
            Pos2::new(
                plot_rect.left() + 4.0,
                (ty - 7.0).max(plot_rect.top() + 4.0),
            ),
            Align2::LEFT_CENTER,
            format!("Alert {}%", thresh as u8),
            FontId::proportional(9.5),
            Color32::from_rgb(248, 113, 113),
        );
    }

    if values.len() >= 2 {
        let n = values.len();
        let mut pts = Vec::with_capacity(n);
        for (idx, &v) in values.iter().enumerate() {
            let x = plot_rect.left() + (idx as f32 / (n - 1) as f32) * plot_rect.width();
            let norm = ((v - min_val) / span).clamp(0.04, 0.96);
            let y = plot_rect.bottom() - norm * plot_rect.height();
            pts.push(Pos2::new(x, y));
        }

        let fill = rgba_premul(color.r(), color.g(), color.b(), 44);
        for w in pts.windows(2) {
            let poly = vec![
                w[0],
                w[1],
                Pos2::new(w[1].x, plot_rect.bottom()),
                Pos2::new(w[0].x, plot_rect.bottom()),
            ];
            painter.add(Shape::convex_polygon(poly, fill, Stroke::NONE));
        }

        // Outer soft glow line + crisp main trendline
        painter.add(Shape::line(
            pts.clone(),
            Stroke::new(4.5_f32, rgba_premul(color.r(), color.g(), color.b(), 50)),
        ));
        painter.add(Shape::line(pts.clone(), Stroke::new(2.2_f32, color)));

        if let Some(&last_pt) = pts.last() {
            painter.circle_filled(
                last_pt,
                5.5,
                rgba_premul(color.r(), color.g(), color.b(), 90),
            );
            painter.circle_filled(last_pt, 3.0, Color32::WHITE);
        }
    }
}

/// Draws the clickable Settings accordion toggle button (with animated chevron icon + summary pill).
pub fn draw_settings_toggle_header(ui: &mut Ui, expanded: bool, summary_text: &str) -> Response {
    let desired = Vec2::new(300.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    let painter = ui.painter();

    // Chevron circle badge
    let chev_center = Pos2::new(rect.left() + 12.0, rect.center().y);
    let badge_bg = if response.hovered() {
        Color32::from_rgb(38, 56, 88)
    } else {
        Color32::from_rgb(28, 42, 68)
    };
    painter.circle_filled(chev_center, 11.0, badge_bg);
    painter.circle_stroke(
        chev_center,
        11.0,
        Stroke::new(
            1.0_f32,
            if response.hovered() {
                Color32::from_rgb(96, 165, 250)
            } else {
                Color32::from_rgb(60, 82, 118)
            },
        ),
    );

    let chev_stroke = Stroke::new(1.8_f32, Color32::from_rgb(56, 189, 248));
    if expanded {
        // Downward chevron `v`
        painter.line_segment(
            [
                Pos2::new(chev_center.x - 4.0, chev_center.y - 1.5),
                Pos2::new(chev_center.x, chev_center.y + 2.5),
            ],
            chev_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(chev_center.x, chev_center.y + 2.5),
                Pos2::new(chev_center.x + 4.0, chev_center.y - 1.5),
            ],
            chev_stroke,
        );
    } else {
        // Rightward chevron `>`
        painter.line_segment(
            [
                Pos2::new(chev_center.x - 1.5, chev_center.y - 4.0),
                Pos2::new(chev_center.x + 2.5, chev_center.y),
            ],
            chev_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(chev_center.x + 2.5, chev_center.y),
                Pos2::new(chev_center.x - 1.5, chev_center.y + 4.0),
            ],
            chev_stroke,
        );
    }

    painter.text(
        Pos2::new(rect.left() + 30.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Settings",
        FontId::proportional(16.0),
        Color32::from_rgb(248, 250, 252),
    );

    painter.text(
        Pos2::new(rect.left() + 98.0, rect.center().y + 0.5),
        Align2::LEFT_CENTER,
        summary_text,
        FontId::proportional(11.5),
        Color32::from_rgb(148, 163, 184),
    );

    response
}

/// Draws a modern Windows 11 / iOS style pill toggle switch (`true` = sky blue ON, `false` = slate OFF).
pub fn draw_pill_toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let desired_size = Vec2::new(38.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    paint_pill_toggle_at(ui.painter(), rect, *on, response.hovered());
    response
}

/// Paints a pill toggle switch inside the given `rect`.
pub fn paint_pill_toggle_at(painter: &egui::Painter, rect: Rect, on: bool, hovered: bool) {
    let radius = rect.height() * 0.5;
    let bg_fill = if on {
        Color32::from_rgb(96, 165, 250)
    } else if hovered {
        Color32::from_rgb(38, 50, 72)
    } else {
        Color32::from_rgb(24, 34, 52)
    };

    let border_color = if on {
        Color32::from_rgb(147, 197, 253)
    } else {
        Color32::from_rgb(71, 85, 105)
    };

    painter.rect_filled(rect, Rounding::same(radius), bg_fill);
    painter.rect_stroke(
        rect,
        Rounding::same(radius),
        Stroke::new(1.0_f32, border_color),
    );

    let thumb_r = radius - 3.0;
    let thumb_x = if on {
        rect.right() - radius
    } else {
        rect.left() + radius
    };
    let thumb_center = Pos2::new(thumb_x, rect.center().y);

    let thumb_color = if on {
        Color32::from_rgb(15, 23, 42)
    } else {
        Color32::from_rgb(100, 116, 139)
    };

    painter.circle_filled(thumb_center, thumb_r, thumb_color);
}

/// Draws the interactive 10%-increment stepped threshold bar + pill labels (10% .. 100%)
/// matching the mockup design.
pub fn draw_threshold_stepper(ui: &mut Ui, threshold_percent: &mut u8) -> bool {
    let mut changed = false;
    let total_w = ui.available_width();
    let total_h = 46.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::click_and_drag());
    let painter = ui.painter();

    let pad_x = 22.0;
    let track_left = rect.left() + pad_x;
    let track_right = rect.right() - pad_x;
    let track_w = (track_right - track_left).max(10.0);
    let track_y = rect.top() + 10.0;

    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let rel = ((pos.x - track_left) / track_w).clamp(0.0, 1.0);
            let step_idx = (rel * 9.0).round() as u8;
            let new_val = ((step_idx + 1) * 10).clamp(10, 100);
            if *threshold_percent != new_val {
                *threshold_percent = new_val;
                changed = true;
            }
        }
    }

    let active_idx = (((*threshold_percent).clamp(10, 100) / 10) - 1) as usize;
    let active_x = track_left + (active_idx as f32 / 9.0) * track_w;

    painter.line_segment(
        [
            Pos2::new(track_left, track_y),
            Pos2::new(track_right, track_y),
        ],
        Stroke::new(3.5_f32, Color32::from_rgb(71, 85, 105)),
    );

    let sky_blue = Color32::from_rgb(56, 189, 248);
    painter.line_segment(
        [Pos2::new(track_left, track_y), Pos2::new(active_x, track_y)],
        Stroke::new(3.5_f32, sky_blue),
    );

    for i in 0..10 {
        let step_val = ((i + 1) * 10) as u8;
        let x = track_left + (i as f32 / 9.0) * track_w;
        let node_pos = Pos2::new(x, track_y);
        let is_selected = i == active_idx;
        let is_passed = i <= active_idx;

        if is_selected {
            painter.circle_stroke(
                node_pos,
                8.5,
                Stroke::new(2.0_f32, rgba_premul(56, 189, 248, 95)),
            );
            painter.circle_filled(node_pos, 6.2, sky_blue);
            painter.circle_stroke(
                node_pos,
                6.2,
                Stroke::new(1.5_f32, Color32::from_rgb(224, 242, 254)),
            );
        } else if is_passed {
            painter.circle_filled(node_pos, 4.0, sky_blue);
        } else {
            painter.circle_filled(node_pos, 4.0, Color32::from_rgb(100, 116, 139));
        }

        let label_y = rect.top() + 33.0;
        let label_text = format!("{}%", step_val);
        if is_selected {
            let pill_rect = Rect::from_center_size(Pos2::new(x, label_y), Vec2::new(42.0, 21.0));
            painter.rect_filled(pill_rect, Rounding::same(6.0), sky_blue);
            painter.text(
                pill_rect.center(),
                Align2::CENTER_CENTER,
                label_text,
                FontId::proportional(12.0),
                Color32::from_rgb(11, 17, 30),
            );
        } else {
            painter.text(
                Pos2::new(x, label_y),
                Align2::CENTER_CENTER,
                label_text,
                FontId::proportional(12.0),
                Color32::from_rgb(203, 213, 225),
            );
        }
    }

    changed
}

/// Draws one of the 4 selectable Action Cards with a vector icon, label, and right-aligned pill toggle.
pub fn draw_action_card(
    ui: &mut Ui,
    width: f32,
    icon: VectorIcon,
    label: &str,
    selected: bool,
) -> bool {
    let height = 38.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let painter = ui.painter();

    let bg_fill = if selected {
        Color32::from_rgb(28, 40, 64)
    } else if response.hovered() {
        Color32::from_rgb(25, 36, 56)
    } else {
        Color32::from_rgb(20, 29, 46)
    };

    let border_stroke = if selected {
        Stroke::new(1.2_f32, Color32::from_rgb(71, 120, 180))
    } else if response.hovered() {
        Stroke::new(1.0_f32, Color32::from_rgb(60, 78, 108))
    } else {
        Stroke::new(1.0_f32, Color32::from_rgb(42, 56, 80))
    };

    painter.rect_filled(rect, Rounding::same(8.0), bg_fill);
    painter.rect_stroke(rect, Rounding::same(8.0), border_stroke);

    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        Vec2::new(18.0, 18.0),
    );
    let icon_color = if selected {
        Color32::from_rgb(241, 245, 249)
    } else {
        Color32::from_rgb(165, 180, 202)
    };
    paint_vector_icon(painter, icon_rect, icon, icon_color);

    painter.text(
        Pos2::new(rect.left() + 36.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.2),
        Color32::from_rgb(232, 238, 248),
    );

    let toggle_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 27.0, rect.center().y),
        Vec2::new(36.0, 19.0),
    );
    paint_pill_toggle_at(painter, toggle_rect, selected, response.hovered());

    response.clicked()
}

/// Draws a sleek button with a custom vector icon on the left (e.g., Save Settings, Test Action).
pub fn draw_icon_button(
    ui: &mut Ui,
    icon: Option<VectorIcon>,
    label: &str,
    accent: bool,
) -> Response {
    let font = FontId::proportional(13.0);
    let text_color = Color32::from_rgb(241, 245, 249);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_color);

    let icon_space = if icon.is_some() { 22.0 } else { 0.0 };
    let btn_size = Vec2::new(galley.size().x + icon_space + 26.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(btn_size, Sense::click());
    let painter = ui.painter();

    let bg = if response.is_pointer_button_down_on() {
        Color32::from_rgb(30, 58, 95)
    } else if response.hovered() {
        if accent {
            Color32::from_rgb(38, 56, 88)
        } else {
            Color32::from_rgb(35, 47, 70)
        }
    } else if accent {
        Color32::from_rgb(28, 42, 68)
    } else {
        Color32::from_rgb(24, 34, 52)
    };

    let border = if response.hovered() {
        Color32::from_rgb(96, 165, 250)
    } else if accent {
        Color32::from_rgb(68, 98, 148)
    } else {
        Color32::from_rgb(51, 65, 85)
    };

    painter.rect_filled(rect, Rounding::same(7.0), bg);
    painter.rect_stroke(rect, Rounding::same(7.0), Stroke::new(1.0_f32, border));

    let mut text_x = rect.left() + 13.0;
    if let Some(ic) = icon {
        let ic_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 18.0, rect.center().y),
            Vec2::new(15.0, 15.0),
        );
        paint_vector_icon(painter, ic_rect, ic, Color32::from_rgb(186, 206, 235));
        text_x += 20.0;
    }

    painter.galley(
        Pos2::new(text_x, rect.center().y - galley.size().y * 0.5),
        galley,
        text_color,
    );

    response
}
