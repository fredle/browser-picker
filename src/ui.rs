//! The window: profile picker, rules manager, and default-browser setup guide.

use crate::default_browser as db;
use crate::profiles::Profile;
use crate::rules::{self, Rule};
use crate::{avatar, install, launch, monitor, theme};
use eframe::egui;
use std::time::{Duration, Instant};

const CARD_H: f32 = 56.0;
const CARD_GAP: f32 = 4.0;
const AVATAR: f32 = 36.0;
const PICKER_W: f32 = 470.0;
const RULES_W: f32 = 540.0;
const RULES_H: f32 = 580.0;
const MAX_VISIBLE: usize = 9;
const HEADER_H: f32 = 62.0;
const FOOTER_H: f32 = 34.0;
const PAD: f32 = 16.0;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Screen {
    Picker,
    Rules,
}

enum Action {
    None,
    Select(usize),
    SetDefault(usize),
    RemoveDefault,
    Manage,
}

fn picker_size(n: usize) -> (f32, f32) {
    let rows = n.clamp(1, MAX_VISIBLE) as f32;
    let list = rows * (CARD_H + CARD_GAP) + 12.0;
    (PICKER_W, HEADER_H + 1.0 + list + 1.0 + FOOTER_H)
}

fn window_size(screen: Screen, n: usize) -> (f32, f32) {
    match screen {
        Screen::Picker => picker_size(n),
        Screen::Rules => (RULES_W, RULES_H),
    }
}

pub fn run(screen: Screen, url: Option<String>) -> eframe::Result<()> {
    let profiles = crate::profiles::discover();
    let (w, h) = window_size(screen, profiles.len());

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Browser Picker")
        .with_inner_size([w, h])
        .with_resizable(false)
        .with_always_on_top();
    if let Some(pos) = monitor::centered_pos(w, h) {
        viewport = viewport.with_position(pos);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Browser Picker",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screen, url, profiles)))),
    )
}

pub struct App {
    screen: Screen,
    url: Option<String>,
    profiles: Vec<Profile>,
    rules: Vec<Rule>,
    textures: Vec<Option<egui::TextureHandle>>,
    bold: egui::FontFamily,
    textures_loaded: bool,

    form_pattern: String,
    form_kind: String,
    form_profile: usize,
    /// (pattern, kind) of the rule being edited, identifying it in the list.
    editing: Option<(String, String)>,

    status: db::Status,
    status_checked: Instant,
    default_name: Option<String>,

    resize_to: Option<(f32, f32)>,

    settings: crate::settings::Settings,
    /// Real destination of a wrapped link (Mimecast, SafeLinks, ...), filled
    /// in asynchronously once `resolve_rx` reports back. `None` until then,
    /// and stays `None` if the link isn't wrapped or resolution fails.
    resolved_domain: Option<String>,
    resolve_rx: Option<std::sync::mpsc::Receiver<Option<String>>>,

    /// Checked once per window, on first reaching the rules screen.
    update_checked: bool,
    update_check_rx: Option<std::sync::mpsc::Receiver<Option<velopack::UpdateInfo>>>,
    update_available: Option<velopack::UpdateInfo>,
    update_applying: bool,
    update_apply_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    update_apply_error: Option<String>,
    version: String,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        screen: Screen,
        url: Option<String>,
        profiles: Vec<Profile>,
    ) -> Self {
        let bold = install_fonts(&cc.egui_ctx);
        apply_visuals(&cc.egui_ctx);

        let seed_pattern = url.as_deref().map(rules::domain_of).unwrap_or_default();
        let n = profiles.len();
        let settings = crate::settings::load();

        // Kick off resolution in the background so the window still appears
        // immediately; the picker just updates itself once (if) it lands.
        let resolve_rx = if screen == Screen::Picker && settings.unwrap_wrapped_links {
            url.as_deref()
                .filter(|u| crate::unwrap::is_wrapped_host(&rules::domain_of(u)))
                .map(|u| {
                    let u = u.to_string();
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(crate::unwrap::resolve(&u));
                    });
                    rx
                })
        } else {
            None
        };

        let mut app = Self {
            screen,
            url,
            profiles,
            rules: rules::load(),
            textures: vec![None; n],
            bold,
            textures_loaded: false,
            form_pattern: seed_pattern,
            form_kind: "domain".to_string(),
            form_profile: 0,
            editing: None,
            status: db::status(),
            status_checked: Instant::now(),
            default_name: db::current_default_name(),
            resize_to: None,
            settings,
            resolved_domain: None,
            resolve_rx,
            update_checked: false,
            update_check_rx: None,
            update_available: None,
            update_applying: false,
            update_apply_rx: None,
            update_apply_error: None,
            version: crate::update::current_version(),
        };
        if app.screen == Screen::Rules {
            app.start_update_check();
        }
        app
    }

    /// Kick off a background update check the first time the rules screen is
    /// reached (from either startup or `Action::Manage`). Silent: an offline
    /// check, or a dev build with nothing for `UpdateManager` to find, just
    /// leaves `update_available` as `None`.
    fn start_update_check(&mut self) {
        if self.update_checked {
            return;
        }
        self.update_checked = true;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::update::check());
        });
        self.update_check_rx = Some(rx);
    }

    fn poll_update(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.update_check_rx {
            match rx.try_recv() {
                Ok(info) => {
                    self.update_available = info;
                    self.update_check_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(300));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.update_check_rx = None;
                }
            }
        }
        if let Some(rx) = &self.update_apply_rx {
            match rx.try_recv() {
                Ok(Ok(())) => {} // process is about to be replaced
                Ok(Err(e)) => {
                    self.update_applying = false;
                    self.update_apply_error = Some(e);
                    self.update_apply_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(300));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.update_applying = false;
                    self.update_apply_rx = None;
                }
            }
        }
    }

    fn apply_update(&mut self) {
        let Some(info) = self.update_available.clone() else {
            return;
        };
        self.update_applying = true;
        self.update_apply_error = None;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::update::install_and_restart(&info));
        });
        self.update_apply_rx = Some(rx);
    }

    /// The domain rules should match and the context menu should offer,
    /// preferring a resolved wrapper destination over the wrapper host.
    fn effective_domain(&self) -> String {
        self.resolved_domain
            .clone()
            .unwrap_or_else(|| rules::domain_of(self.url.as_deref().unwrap_or("")))
    }

    /// Poll the background resolver, if one is running, without blocking.
    fn poll_resolve(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.resolve_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.resolved_domain = result.as_deref().map(rules::domain_of);
                self.resolve_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.resolve_rx = None;
            }
        }
    }

    fn font(&self, size: f32, bold: bool) -> egui::FontId {
        egui::FontId::new(
            size,
            if bold {
                self.bold.clone()
            } else {
                egui::FontFamily::Proportional
            },
        )
    }

    fn load_textures(&mut self, ctx: &egui::Context) {
        if self.textures_loaded {
            return;
        }
        self.textures_loaded = true;
        for (i, p) in self.profiles.iter().enumerate() {
            if let Some(path) = &p.image_path {
                self.textures[i] = avatar::load(ctx, path, AVATAR as usize * 2);
            }
        }
    }

    /// Re-read the association from the registry now and then, so the banner
    /// updates by itself once the user comes back from Windows Settings.
    fn refresh_status(&mut self, ctx: &egui::Context) {
        if self.status_checked.elapsed() > Duration::from_millis(600) {
            self.status = db::status();
            self.default_name = db::current_default_name();
            self.status_checked = Instant::now();
        }
        if !self.status.is_ok() {
            ctx.request_repaint_after(Duration::from_millis(600));
        }
    }

    fn go_to(&mut self, screen: Screen) {
        self.screen = screen;
        self.resize_to = Some(window_size(screen, self.profiles.len()));
    }

    fn select(&mut self, idx: usize, ctx: &egui::Context) {
        if let (Some(url), Some(profile)) = (self.url.clone(), self.profiles.get(idx)) {
            launch::launch(profile, &url);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Profiles offered in the add/edit form. Private ones are included so an
    /// existing Incognito/InPrivate rule (creatable by right-clicking that card)
    /// can be edited without losing its target.
    fn form_options(&self) -> Vec<usize> {
        (0..self.profiles.len()).collect()
    }

    fn begin_edit(&mut self, rule: &Rule) {
        self.form_pattern = rule.pattern.clone();
        self.form_kind = rule.kind.clone();
        if let Some(slot) = self
            .form_options()
            .iter()
            .position(|&pi| rules::profile_matches(&self.profiles[pi], Some(rule)))
        {
            self.form_profile = slot;
        }
        self.editing = Some((rule.pattern.clone(), rule.kind.clone()));
    }

    fn cancel_edit(&mut self) {
        self.editing = None;
        self.form_pattern.clear();
        self.form_kind = "domain".to_string();
    }

    fn is_editing(&self, rule: &Rule) -> bool {
        self.editing
            .as_ref()
            .is_some_and(|(p, k)| *p == rule.pattern && *k == rule.kind)
    }

    fn label_for_rule(&self, rule: &Rule) -> String {
        match self
            .profiles
            .iter()
            .find(|p| rules::profile_matches(p, Some(rule)))
        {
            Some(p) => p.label(),
            None => format!("{} (not installed)", theme::browser_label(&rule.browser)),
        }
    }

    // ── Picker ────────────────────────────────────────────────────────────

    fn draw_avatar(&self, painter: &egui::Painter, center: egui::Pos2, idx: usize) {
        let profile = &self.profiles[idx];
        let r = AVATAR / 2.0;

        if profile.private {
            painter.circle_filled(center, r, theme::LOCK_BG);
            let body_w = AVATAR * 0.36;
            let body_h = AVATAR * 0.28;
            let body = egui::Rect::from_center_size(
                egui::pos2(center.x, center.y + AVATAR * 0.11),
                egui::vec2(body_w, body_h),
            );
            painter.rect_filled(body, egui::CornerRadius::same(2), egui::Color32::WHITE);
            // Shackle: upper half circle sitting on the body.
            let sr = body_w * 0.36;
            let sc = egui::pos2(center.x, body.top());
            let pts: Vec<egui::Pos2> = (0..=18)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + i as f32 / 18.0);
                    egui::pos2(sc.x + sr * a.cos(), sc.y + sr * a.sin())
                })
                .collect();
            painter.add(egui::Shape::line(
                pts,
                egui::Stroke::new(2.5, egui::Color32::WHITE),
            ));
            return;
        }

        if let Some(Some(tex)) = self.textures.get(idx) {
            let rect = egui::Rect::from_center_size(center, egui::vec2(AVATAR, AVATAR));
            painter.image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            return;
        }

        let color = theme::AVATAR_COLORS[idx % theme::AVATAR_COLORS.len()];
        painter.circle_filled(center, r, color);
        let initial = profile
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            initial,
            self.font(16.0, true),
            egui::Color32::WHITE,
        );
    }

    fn draw_card(
        &self,
        ui: &mut egui::Ui,
        idx: usize,
        domain: &str,
        domain_rule: Option<&Rule>,
    ) -> Action {
        let profile = &self.profiles[idx];
        let is_default = rules::profile_matches(profile, domain_rule);

        // Allocate the space, then interact with an explicit, frame-stable Id.
        // Auto-generated Ids are what the context-menu popup state is keyed on,
        // so they have to be predictable.
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), CARD_H),
            egui::Sense::hover(),
        );
        let resp = ui.interact(rect, egui::Id::new(("card", idx)), egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let (bg, border) = if resp.hovered() {
                (theme::BG_HOVER, theme::BORDER_STRONG)
            } else {
                (theme::BG_CARD, theme::BORDER)
            };
            let radius = egui::CornerRadius::same(theme::RADIUS_CARD);
            painter.rect_filled(rect, radius, bg);
            painter.rect_stroke(
                rect,
                radius,
                egui::Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );

            // Keyboard shortcut badge
            let shortcut = match idx {
                0..=8 => (idx + 1).to_string(),
                9 => "0".to_string(),
                _ => String::new(),
            };
            painter.text(
                egui::pos2(rect.left() + 14.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                shortcut,
                self.font(11.0, false),
                theme::FG_DIM,
            );

            self.draw_avatar(
                painter,
                egui::pos2(rect.left() + 30.0 + AVATAR / 2.0, rect.center().y),
                idx,
            );

            let text_x = rect.left() + 30.0 + AVATAR + 12.0;
            painter.text(
                egui::pos2(text_x, rect.center().y - 8.0),
                egui::Align2::LEFT_CENTER,
                &profile.name,
                self.font(13.0, true),
                theme::FG,
            );

            let mut sub = theme::browser_label(&profile.browser).to_string();
            if is_default {
                sub = if sub.is_empty() {
                    "Default".to_string()
                } else {
                    format!("{sub}  \u{b7}  Default")
                };
            }
            painter.text(
                egui::pos2(text_x, rect.center().y + 9.0),
                egui::Align2::LEFT_CENTER,
                sub,
                self.font(10.5, false),
                theme::FG_DIM,
            );

            // Browser accent bar
            let bar = egui::Rect::from_min_max(
                egui::pos2(rect.right() - 7.0, rect.top() + 10.0),
                egui::pos2(rect.right() - 4.0, rect.bottom() - 10.0),
            );
            painter.rect_filled(
                bar,
                egui::CornerRadius::same(2),
                theme::accent(&profile.browser),
            );
        }

        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let mut action = if resp.clicked() {
            Action::Select(idx)
        } else {
            Action::None
        };

        // Anchor the menu to the card, not the pointer. The pointer-anchored
        // variant (Response::context_menu) resolves its position from
        // pointer_hover_pos, and silently renders nothing when that is absent.
        egui::Popup::menu(&resp)
            .open_memory(if resp.secondary_clicked() {
                Some(egui::SetOpenCommand::Bool(true))
            } else if resp.clicked() {
                Some(egui::SetOpenCommand::Bool(false))
            } else {
                None
            })
            .show(|ui| {
                if !domain.is_empty() {
                    if is_default {
                        if ui.button(format!("Remove default for {domain}")).clicked() {
                            action = Action::RemoveDefault;
                        }
                    } else if ui
                        .button(format!("Always use {} for {}", profile.name, domain))
                        .clicked()
                    {
                        action = Action::SetDefault(idx);
                    }
                    ui.separator();
                }
                if ui.button("Manage defaults\u{2026}").clicked() {
                    action = Action::Manage;
                }
            });

        action
    }

    fn picker(&mut self, ui: &mut egui::Ui) -> Action {
        let mut action = Action::None;
        let url = self.url.clone().unwrap_or_default();
        let domain = self.effective_domain();
        let domain_rule = rules::find_domain_rule(&domain, &self.rules);

        // Header
        let (rect, header_resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), HEADER_H),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        painter.text(
            egui::pos2(rect.left() + PAD, rect.top() + 18.0),
            egui::Align2::LEFT_CENTER,
            "Open link in\u{2026}",
            self.font(15.0, true),
            theme::FG,
        );
        // A resolved wrapper destination replaces the raw (opaque, wrapper-
        // hosted) URL in the subtitle - the wrapped URL is still what gets
        // launched, and stays available as a hover tooltip so nothing is
        // hidden, just made readable.
        let (subtitle, subtitle_color, tooltip) = if let Some(resolved) = &self.resolved_domain {
            (format!("Opens: {resolved}"), theme::FG, Some(url.clone()))
        } else if self.resolve_rx.is_some() {
            (
                "Resolving protected link\u{2026}".to_string(),
                theme::FG_DIM,
                None,
            )
        } else {
            let shown = if url.chars().count() <= 60 {
                url.clone()
            } else {
                format!("{}\u{2026}", url.chars().take(57).collect::<String>())
            };
            (shown, theme::FG_DIM, None)
        };
        painter.text(
            egui::pos2(rect.left() + PAD, rect.top() + 40.0),
            egui::Align2::LEFT_CENTER,
            subtitle,
            self.font(11.0, false),
            subtitle_color,
        );
        if let Some(tooltip) = tooltip {
            header_resp.on_hover_text(tooltip);
        }
        separator(ui);

        // Not-the-default warning, since links won't reach us at all
        if !self.status.is_ok() {
            let (rect, resp) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::click());
            let painter = ui.painter();
            painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::BG_CARD);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - 1.0),
                    rect.right_bottom(),
                ),
                egui::CornerRadius::ZERO,
                theme::BORDER,
            );
            painter.text(
                egui::pos2(rect.left() + PAD, rect.center().y),
                egui::Align2::LEFT_CENTER,
                "Not your default browser \u{2014} click to set up",
                self.font(10.5, false),
                theme::WARN,
            );
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                action = Action::Manage;
            }
        }

        // Profile list
        if self.profiles.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No Chrome or Edge profiles found.")
                        .color(theme::FG_DIM)
                        .font(self.font(12.0, false)),
                );
            });
        } else {
            let list_h = ui.available_height() - FOOTER_H - 1.0;
            egui::ScrollArea::vertical()
                .max_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = CARD_GAP;
                            ui.set_width(ui.available_width() - 10.0);
                            for idx in 0..self.profiles.len() {
                                let a = self.draw_card(ui, idx, &domain, domain_rule.as_ref());
                                if !matches!(a, Action::None) {
                                    action = a;
                                }
                            }
                        });
                    });
                    ui.add_space(6.0);
                });
        }

        // Footer
        separator(ui);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), FOOTER_H),
            egui::Sense::hover(),
        );
        let hint = if self.profiles.len() > MAX_VISIBLE {
            "1\u{2013}9 to pick  \u{b7}  scroll for more  \u{b7}  right-click to set a default  \u{b7}  Esc to cancel"
        } else {
            "1\u{2013}9 to pick  \u{b7}  right-click to set a default  \u{b7}  Esc to cancel"
        };
        ui.painter().text(
            egui::pos2(rect.left() + PAD, rect.center().y),
            egui::Align2::LEFT_CENTER,
            hint,
            self.font(9.5, false),
            theme::FG_DIM,
        );

        let link = "Manage defaults\u{2026}";
        let galley =
            ui.painter()
                .layout_no_wrap(link.to_string(), self.font(9.5, false), theme::FG_DIM);
        let link_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.right() - PAD - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley.size(),
        );
        let link_resp = ui.interact(
            link_rect,
            egui::Id::new("manage_link"),
            egui::Sense::click(),
        );
        ui.painter().galley(
            link_rect.min,
            galley,
            if link_resp.hovered() {
                theme::FG
            } else {
                theme::FG_DIM
            },
        );
        if link_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if link_resp.clicked() {
            action = Action::Manage;
        }

        action
    }

    // ── Rules manager ─────────────────────────────────────────────────────

    fn setup_banner(&mut self, ui: &mut egui::Ui) {
        let missing = db::missing_schemes();
        let current = self.default_name.clone();

        let (accent, title, body) = match self.status {
            db::Status::Default => (
                theme::OK,
                "Browser Picker is your default browser".to_string(),
                "Every link you click will come here first.".to_string(),
            ),
            db::Status::Partial => {
                // Partial always means exactly one scheme, but never build a
                // sentence with a hole in it if that ever changes.
                let schemes = if missing.is_empty() {
                    "some".to_string()
                } else {
                    missing.join(" and ")
                };
                let owner = current
                    .clone()
                    .map(|c| format!(" to {c}"))
                    .unwrap_or_default();
                (
                    theme::WARN,
                    "Only partly set as your default".to_string(),
                    format!(
                        "Windows still sends {schemes} links{owner}. \
                         Set Browser Picker for both HTTP and HTTPS."
                    ),
                )
            }
            db::Status::NotDefault => match current.clone() {
                Some(name) => (
                    theme::WARN,
                    format!("{name} is your default browser"),
                    "Links will open there instead of showing the picker.".to_string(),
                ),
                None => (
                    theme::WARN,
                    "No default browser is set".to_string(),
                    "Set Browser Picker as your default so links come here.".to_string(),
                ),
            },
            db::Status::NotRegistered => (
                theme::BAD,
                "Not registered with Windows".to_string(),
                "Windows doesn't list Browser Picker yet, so it can't be chosen as your default."
                    .to_string(),
            ),
        };

        let frame = egui::Frame {
            fill: theme::BG_CARD,
            inner_margin: egui::Margin::same(12),
            corner_radius: egui::CornerRadius::same(theme::RADIUS_CARD),
            stroke: egui::Stroke::new(1.0, theme::BORDER),
            ..Default::default()
        };

        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let dot = ui.allocate_space(egui::vec2(10.0, 10.0)).1;
                ui.painter().circle_filled(dot.center(), 4.0, accent);
                ui.label(
                    egui::RichText::new(title)
                        .color(theme::FG)
                        .font(self.font(11.5, true)),
                );
            });
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(body)
                    .color(theme::FG_DIM)
                    .font(self.font(10.5, false)),
            );

            if self.status.is_ok() {
                return;
            }

            ui.add_space(8.0);
            if self.status == db::Status::NotRegistered {
                ui.label(
                    egui::RichText::new("1.  Register Browser Picker with Windows.\n2.  Open Default apps and pick it for HTTP and HTTPS.")
                        .color(theme::FG_DIM)
                        .font(self.font(10.5, false)),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Register now").clicked() {
                        let _ = install::register();
                        self.status = db::status();
                        self.status_checked = Instant::now();
                    }
                });
            } else {
                ui.label(
                    egui::RichText::new(
                        "Windows only allows this by hand:\n\
                         1.  Open Default apps below.\n\
                         2.  Find Browser Picker in the list.\n\
                         3.  Set it for HTTP and HTTPS.",
                    )
                    .color(theme::FG_DIM)
                    .font(self.font(10.5, false)),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Open Windows Default apps").clicked() {
                        db::open_default_apps_settings();
                    }
                    ui.label(
                        egui::RichText::new("this page updates itself once you're done")
                            .color(theme::FG_DIM)
                            .font(self.font(9.5, false)),
                    );
                });
            }
        });
    }

    fn update_banner(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame {
            fill: theme::BG_CARD,
            inner_margin: egui::Margin::same(12),
            corner_radius: egui::CornerRadius::same(theme::RADIUS_CARD),
            stroke: egui::Stroke::new(1.0, theme::ACCENT),
            ..Default::default()
        };
        let mut apply_clicked = false;
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(err) = &self.update_apply_error {
                ui.label(
                    egui::RichText::new(format!("Couldn't install the update: {err}"))
                        .color(theme::BAD)
                        .font(self.font(11.0, false)),
                );
                return;
            }
            let Some(info) = &self.update_available else {
                return;
            };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Browser Picker {} is available",
                        info.TargetFullRelease.Version
                    ))
                    .color(theme::FG)
                    .font(self.font(11.5, true)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.update_applying {
                        "Updating\u{2026}"
                    } else {
                        "Update & restart"
                    };
                    if ui
                        .add_enabled(!self.update_applying, egui::Button::new(label))
                        .clicked()
                    {
                        apply_clicked = true;
                    }
                });
            });
        });
        if apply_clicked {
            self.apply_update();
        }
    }

    fn rules_screen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("Default browser rules")
                .color(theme::FG)
                .font(self.font(15.0, true)),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(format!(
                "Matching links open automatically. Hold {} while clicking a link to choose instead.",
                crate::bypass::KEY_NAME
            ))
            .color(theme::FG_DIM)
            .font(self.font(10.5, false)),
        );

        ui.add_space(10.0);
        self.setup_banner(ui);
        ui.add_space(10.0);

        if self.update_available.is_some() || self.update_apply_error.is_some() {
            self.update_banner(ui);
            ui.add_space(10.0);
        }

        let mut unwrap_enabled = self.settings.unwrap_wrapped_links;
        if ui
            .checkbox(
                &mut unwrap_enabled,
                egui::RichText::new("Resolve protected links (Mimecast, SafeLinks, Proofpoint\u{2026}) to their real destination")
                    .font(self.font(11.0, false))
                    .color(theme::FG),
            )
            .on_hover_text(
                "Shows and matches rules against the real site behind a wrapped link.\n\
                 Sends a request to the sender's link-protection service to resolve it,\n\
                 which registers as a click against the original email link.",
            )
            .changed()
        {
            self.settings.unwrap_wrapped_links = unwrap_enabled;
            crate::settings::save(&self.settings);
        }
        ui.add_space(10.0);

        // Existing rules
        let mut remove: Option<(String, String)> = None;
        let mut edit_request: Option<Rule> = None;
        egui::ScrollArea::vertical()
            .max_height(210.0)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if self.rules.is_empty() {
                    ui.label(
                        egui::RichText::new("No rules yet.")
                            .color(theme::FG_DIM)
                            .font(self.font(11.0, false)),
                    );
                }
                for rule in self.rules.clone() {
                    let editing_this = self.is_editing(&rule);
                    let frame = egui::Frame {
                        fill: if editing_this {
                            theme::BG_HOVER
                        } else {
                            theme::BG_CARD
                        },
                        inner_margin: egui::Margin::symmetric(10, 8),
                        corner_radius: egui::CornerRadius::same(theme::RADIUS_SMALL),
                        stroke: egui::Stroke::new(
                            1.0,
                            if editing_this {
                                theme::ACCENT
                            } else {
                                theme::BORDER
                            },
                        ),
                        ..Default::default()
                    };
                    frame.show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let kind = match rule.kind.as_str() {
                                "domain" => "Domain",
                                "pattern" => "Pattern",
                                "url" => "Exact URL",
                                other => other,
                            };
                            ui.label(
                                egui::RichText::new(format!("[{kind}]"))
                                    .color(theme::FG_DIM)
                                    .font(self.font(10.0, false)),
                            );
                            ui.label(
                                egui::RichText::new(&rule.pattern)
                                    .color(theme::FG)
                                    .font(self.font(11.0, true)),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "\u{2192} {}",
                                    self.label_for_rule(&rule)
                                ))
                                .color(theme::FG_DIM)
                                .font(self.font(11.0, false)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(
                                            egui::RichText::new("\u{d7}")
                                                .font(self.font(13.0, true))
                                                .color(theme::FG_DIM),
                                        )
                                        .clicked()
                                    {
                                        remove = Some((rule.pattern.clone(), rule.kind.clone()));
                                    }
                                    if ui
                                        .button(
                                            egui::RichText::new(if editing_this {
                                                "Editing"
                                            } else {
                                                "Edit"
                                            })
                                            .font(self.font(10.0, false))
                                            .color(
                                                if editing_this {
                                                    theme::FG
                                                } else {
                                                    theme::FG_DIM
                                                },
                                            ),
                                        )
                                        .clicked()
                                    {
                                        edit_request = Some(rule.clone());
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(3.0);
                }
            });

        if let Some((pattern, kind)) = remove {
            self.rules = rules::remove(&pattern, &kind);
            if self
                .editing
                .as_ref()
                .is_some_and(|(p, k)| *p == pattern && *k == kind)
            {
                self.cancel_edit();
            }
        }
        if let Some(rule) = edit_request {
            self.begin_edit(&rule);
        }

        ui.add_space(10.0);
        separator(ui);
        ui.add_space(10.0);

        // Add / edit rule
        let editing = self.editing.clone();
        ui.label(
            egui::RichText::new(if editing.is_some() {
                "Edit rule"
            } else {
                "Add rule"
            })
            .color(theme::FG)
            .font(self.font(11.5, true)),
        );
        ui.add_space(6.0);
        let body_font = self.font(11.0, false);
        ui.add(
            egui::TextEdit::singleline(&mut self.form_pattern)
                .desired_width(ui.available_width())
                .hint_text("github.com  or  *://*.atlassian.net/browse/*")
                .font(body_font),
        );
        ui.add_space(6.0);
        let radio_font = self.font(11.0, false);
        ui.horizontal(|ui| {
            for (value, label) in [
                ("domain", "Domain"),
                ("pattern", "Pattern (*)"),
                ("url", "Exact URL"),
            ] {
                ui.radio_value(
                    &mut self.form_kind,
                    value.to_string(),
                    egui::RichText::new(label)
                        .font(radio_font.clone())
                        .color(theme::FG),
                );
            }
        });

        let options = self.form_options();
        if options.is_empty() {
            return;
        }
        if self.form_profile >= options.len() {
            self.form_profile = 0;
        }

        ui.add_space(8.0);
        let combo_font = self.font(11.0, false);
        let selected = self.profiles[options[self.form_profile]].label();
        let option_labels: Vec<String> = options
            .iter()
            .map(|&pi| self.profiles[pi].label())
            .collect();
        let mut cancel = false;
        let mut reset_form = false;
        let reserve = if editing.is_some() { 150.0 } else { 90.0 };
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("profile_pick")
                .selected_text(egui::RichText::new(selected).font(combo_font.clone()))
                .width(ui.available_width() - reserve)
                .show_ui(ui, |ui| {
                    for (slot, label) in option_labels.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.form_profile,
                            slot,
                            egui::RichText::new(label).font(combo_font.clone()),
                        );
                    }
                });
            let commit = if editing.is_some() { "Save" } else { "Add" };
            if ui.button(commit).clicked() {
                let pattern = self.form_pattern.trim().to_string();
                if !pattern.is_empty() {
                    let profile = self.profiles[options[self.form_profile]].clone();
                    let kind = self.form_kind.clone();
                    self.rules = match &editing {
                        // Edits replace in place, preserving match priority.
                        Some((old_pattern, old_kind)) => {
                            rules::update(old_pattern, old_kind, &pattern, &kind, &profile)
                        }
                        None => rules::add(&pattern, &kind, &profile),
                    };
                    // Leaving the edited values in the form invites a stray
                    // "Add" that would re-add the rule at top priority.
                    if editing.is_some() {
                        reset_form = true;
                    }
                    self.editing = None;
                }
            }
            if editing.is_some() && ui.button("Cancel").clicked() {
                cancel = true;
            }
        });
        if cancel || reset_form {
            self.cancel_edit();
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("v{}", self.version))
                    .color(theme::FG_DIM)
                    .font(self.font(9.0, false)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui_root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui_root.ctx().clone();
        let ctx = &ctx;
        self.load_textures(ctx);
        self.refresh_status(ctx);
        self.poll_resolve(ctx);
        self.poll_update(ctx);

        if let Some((w, h)) = self.resize_to.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
            if let Some(pos) = monitor::centered_pos(w, h) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    pos[0], pos[1],
                )));
            }
        }

        // Keyboard: Esc always closes; digits pick a profile in the picker.
        let mut key_pick: Option<usize> = None;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if self.screen == Screen::Picker {
                for (n, key) in [
                    egui::Key::Num1,
                    egui::Key::Num2,
                    egui::Key::Num3,
                    egui::Key::Num4,
                    egui::Key::Num5,
                    egui::Key::Num6,
                    egui::Key::Num7,
                    egui::Key::Num8,
                    egui::Key::Num9,
                ]
                .iter()
                .enumerate()
                {
                    if i.key_pressed(*key) {
                        key_pick = Some(n);
                    }
                }
                if i.key_pressed(egui::Key::Num0) {
                    key_pick = Some(9);
                }
            }
        });
        if let Some(idx) = key_pick {
            if idx < self.profiles.len() {
                self.select(idx, ctx);
                return;
            }
        }

        let frame = egui::Frame {
            fill: theme::BG,
            inner_margin: match self.screen {
                Screen::Picker => egui::Margin::ZERO,
                Screen::Rules => egui::Margin::symmetric(PAD as i8, 0),
            },
            ..Default::default()
        };

        let mut action = Action::None;
        egui::CentralPanel::default()
            .frame(frame)
            .show(ui_root, |ui| match self.screen {
                Screen::Picker => action = self.picker(ui),
                Screen::Rules => self.rules_screen(ui, ctx),
            });

        match action {
            Action::Select(idx) => self.select(idx, ctx),
            Action::SetDefault(idx) => {
                let domain = self.effective_domain();
                let profile = self.profiles[idx].clone();
                self.rules = rules::add(&domain, "domain", &profile);
                self.select(idx, ctx);
            }
            Action::RemoveDefault => {
                let domain = self.effective_domain();
                self.rules = rules::remove(&domain, "domain");
            }
            Action::Manage => {
                self.form_pattern = self.effective_domain();
                self.rules = rules::load();
                self.go_to(Screen::Rules);
                self.start_update_check();
            }
            Action::None => {}
        }
    }
}

fn separator(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, theme::BORDER);
}

/// Use the real Segoe UI so the app looks like the rest of Windows. Falls back
/// to egui's bundled font if the files aren't there.
fn install_fonts(ctx: &egui::Context) -> egui::FontFamily {
    let mut fonts = egui::FontDefinitions::default();
    let mut bold = egui::FontFamily::Proportional;

    if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts.font_data.insert(
            "segoe".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(data)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".to_owned());
    }
    if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf") {
        fonts.font_data.insert(
            "segoe_bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(data)),
        );
        fonts.families.insert(
            egui::FontFamily::Name("bold".into()),
            vec!["segoe_bold".to_owned(), "segoe".to_owned()],
        );
        bold = egui::FontFamily::Name("bold".into());
    }

    ctx.set_fonts(fonts);
    bold
}

fn apply_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = theme::BG;
    v.window_fill = theme::BG_CARD;
    v.extreme_bg_color = theme::BG_CARD;
    v.override_text_color = Some(theme::FG);
    v.window_stroke = egui::Stroke::new(1.0, theme::BORDER);
    v.selection.bg_fill = theme::ACCENT.gamma_multiply(0.30);
    v.selection.stroke = egui::Stroke::new(1.0, theme::ACCENT);

    let hairline = egui::Stroke::new(1.0, theme::BORDER);
    let radius = egui::CornerRadius::same(theme::RADIUS_SMALL);

    v.widgets.noninteractive.bg_fill = theme::BG_CARD;
    v.widgets.noninteractive.weak_bg_fill = theme::BG_CARD;
    v.widgets.noninteractive.bg_stroke = hairline;
    v.widgets.noninteractive.corner_radius = radius;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme::FG_DIM);

    v.widgets.inactive.bg_fill = theme::BG_CARD;
    v.widgets.inactive.weak_bg_fill = theme::BG_CARD;
    v.widgets.inactive.bg_stroke = hairline;
    v.widgets.inactive.corner_radius = radius;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme::FG);

    v.widgets.hovered.bg_fill = theme::BG_HOVER;
    v.widgets.hovered.weak_bg_fill = theme::BG_HOVER;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_STRONG);
    v.widgets.hovered.corner_radius = radius;
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme::FG);

    v.widgets.active.bg_fill = theme::BG_ACTIVE;
    v.widgets.active.weak_bg_fill = theme::BG_ACTIVE;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme::ACCENT);
    v.widgets.active.corner_radius = radius;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme::FG);

    v.widgets.open.bg_fill = theme::BG_HOVER;
    v.widgets.open.weak_bg_fill = theme::BG_HOVER;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_STRONG);
    v.widgets.open.corner_radius = radius;

    // set_visuals() only writes the *currently active* theme's style slot, so
    // popups and menus could otherwise resolve against egui's default and come
    // out mismatched. Pin the preference and fill both slots.
    ctx.set_theme(egui::ThemePreference::Light);
    ctx.set_visuals_of(egui::Theme::Light, v.clone());
    ctx.set_visuals_of(egui::Theme::Dark, v);
}
