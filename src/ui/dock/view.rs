//! La vue d'une zone ancrée : chaque séparation devient un `split` qu'on règle à la
//! souris, chaque groupe une rangée d'onglets au-dessus du panneau montré. Un onglet se
//! tire (`ui::drag`) ; le groupe survolé montre où il se posera.
use super::{
    state::{DockMsg, DockState, Inner},
    tree::{DockNode, DockSide},
};
use crate::{
    application::Action,
    ui::{
        drag::{DragEvent, DragPhase, Payload},
        editor::{EditorPalette, label},
        element::Position,
        element::{
            Align, Axis, Element, ElementId, Insets, Length, PaintContext, Role, Semantics, SizeF,
            Split, TextMeasurer, Widget,
        },
        scene::{RectF, Scene},
        template::Node,
        theme::Color,
    },
};
use std::rc::Rc;

/// Sorte des données que porte un onglet tiré.
const PAYLOAD: &str = "karbon-dock-panel";
/// Largeur de la croix d'un onglet.
const CLOSE: f32 = 16.;

/// Docked panels from a [`DockState`]: groups of tabs side by side or stacked, with
/// dividers to drag. Drag a tab onto another group: its left, right, top or bottom
/// quarter docks it beside that group, its middle (or the row of tabs) adds it as a
/// tab; the zone is shown while dragging. The application keeps the [`DockState`],
/// forwards the [`DockMsg`]s to [`DockState::update`] and saves
/// [`DockTree::to_json`](super::DockTree::to_json) when it likes.
///
/// ```
/// use karbon_ui::ui::{
///     dock::{DockArea, DockMsg, DockSide, DockState, DockTree},
///     element::ElementId,
///     prelude::*,
/// };
///
/// #[derive(Clone)]
/// enum Msg {
///     Dock(DockMsg),
/// }
/// struct Editor {
///     dock: DockState,
/// }
/// impl Application for Editor {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         let names = ["Scène", "Inspecteur", "Console"];
///         DockArea::new(
///             "dock",
///             &self.dock,
///             Msg::Dock,
///             move |panel| names[panel as usize].to_string(),
///             |panel| column(ElementId::named("panel").item(panel), []),
///         )
///         .element()
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         let Msg::Dock(message) = message;
///         self.dock.update(message).rebuild
///     }
/// }
/// let mut tree = DockTree::single(0);
/// tree.dock(1, 0, DockSide::Right);
/// let _ = Editor { dock: DockState::new(tree) };
/// ```
pub struct DockArea<M> {
    id: ElementId,
    state: DockState,
    map: Rc<dyn Fn(DockMsg) -> M>,
    title: Rc<dyn Fn(u64) -> String>,
    content: Rc<dyn Fn(u64) -> Element>,
    closable: bool,
    palette: EditorPalette,
}

impl<M: Clone + 'static> DockArea<M> {
    /// The area of `state`: `map` wraps its gestures in the application's message,
    /// `title` names a panel on its tab, `content` builds the panel shown in a group.
    pub fn new(
        id: impl Into<ElementId>,
        state: &DockState,
        map: impl Fn(DockMsg) -> M + 'static,
        title: impl Fn(u64) -> String + 'static,
        content: impl Fn(u64) -> Element + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            map: Rc::new(map),
            title: Rc::new(title),
            content: Rc::new(content),
            closable: false,
            palette: EditorPalette::DARK,
        }
    }

    /// Show a close button on each tab ([`DockEvent::Closed`](super::DockEvent::Closed)).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Colours of the tabs and marks ([`EditorPalette::DARK`] by default).
    pub fn palette(mut self, palette: EditorPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The element, filling its parent.
    pub fn element(self) -> Element {
        let body = match self.state.tree.root() {
            Some(root) => self.node(root),
            None => Element::column(self.id.child("empty").0, vec![]),
        };
        let mut children = vec![body.height(Length::Fill(1.))];
        if let (Some((_, side)), Some(rect)) = (self.state.aim, self.state.aim_rect) {
            children.push(self.mark(side, rect));
        }
        Element::column(self.id.0, children)
            .width(Length::Fill(1.))
            .height(Length::Fill(1.))
    }

    /// La marque du dépôt visé, posée au-dessus de tout sur la boîte survolée, sans
    /// prendre le pointeur.
    fn mark(&self, side: DockSide, rect: RectF) -> Element {
        let mark = Mark {
            side,
            color: self.palette.accent,
        };
        Node::element(Element::custom(self.id.child("mark").0, mark))
            .position(Position::Fixed)
            .offset(0, Length::Px(rect.top))
            .offset(3, Length::Px(rect.left))
            .width(Length::Px(rect.right - rect.left))
            .height(Length::Px(rect.bottom - rect.top))
            .z_index(i32::MAX - 1)
            .pointer_events(false)
            .build()
    }

    fn node(&self, node: &DockNode) -> Element {
        match node {
            DockNode::Tabs { panels, active } => self.group(panels, *active),
            DockNode::Split {
                axis,
                fraction,
                first,
                second,
            } => {
                let key = (first.first_panel(), second.first_panel());
                let id = self.id.child("split").item(key.0).item(key.1);
                let options = Split {
                    min_first: 60.,
                    min_second: 60.,
                    resizable: true,
                    ..match axis {
                        Axis::Horizontal => Split::horizontal(*fraction),
                        Axis::Vertical => Split::vertical(*fraction),
                    }
                };
                let map = Rc::clone(&self.map);
                Element::split(id.0, options, self.node(first), self.node(second))
                    .on_split(move |share| map(DockMsg(Inner::Share(key, share))))
            }
        }
    }

    /// Un groupe : ses onglets, puis le panneau montré, avec la marque du dépôt visé.
    fn group(&self, panels: &[u64], active: u64) -> Element {
        let id = self.id.child("group").item(active);
        let tabs = panels.iter().map(|&panel| self.tab(panel, panel == active));
        let strip = Element::row(id.child("tabs").0, tabs.collect())
            .height(Length::Px(self.palette.row_height + 4.))
            .surface(self.palette.field, 0.);
        let strip = self.target(strip, active, |_| DockSide::Center);
        let content = (self.content)(active)
            .width(Length::Fill(1.))
            .height(Length::Fill(1.));
        let body = Element::column(id.child("body").0, vec![content])
            .width(Length::Fill(1.))
            .height(Length::Fill(1.));
        let body = self.target(body, active, side_of);
        Element::column(id.0, vec![strip, body])
            .width(Length::Fill(1.))
            .height(Length::Fill(1.))
    }

    /// `element` reçoit les onglets tirés de cette zone, au côté que donne `side`.
    fn target(
        &self,
        element: Element,
        active: u64,
        side: impl Fn(&DragEvent) -> DockSide + Clone + 'static,
    ) -> Element {
        let area = self.id.0;
        let (over, drop) = (Rc::clone(&self.map), Rc::clone(&self.map));
        let side_drop = side.clone();
        element
            .on_drag_over(move |e: &DragEvent| match e.phase {
                DragPhase::Over => {
                    dragged(e, area)?;
                    Some(over(DockMsg(Inner::Aim(Some((
                        active,
                        side(e),
                        box_of(e),
                    ))))))
                }
                DragPhase::Leave => Some(over(DockMsg(Inner::Aim(None)))),
                _ => None,
            })
            .on_drop(move |e: &DragEvent| {
                let panel = dragged(e, area)?;
                Some(drop(DockMsg(Inner::Drop {
                    panel,
                    target: active,
                    side: side_drop(e),
                })))
            })
    }

    /// Un onglet : son nom, sa croix, cliquable et qu'on tire.
    fn tab(&self, panel: u64, active: bool) -> Element {
        let id = self.id.child("tab").item(panel);
        let title = (self.title)(panel);
        let mut children = vec![label(id.child("name"), title.clone(), &self.palette)];
        if self.closable {
            let close = Element::custom(
                id.child("close").0,
                CloseButton {
                    action: Action::new((self.map)(DockMsg(Inner::Close(panel)))),
                    color: self.palette.muted,
                },
            )
            .width(Length::Px(CLOSE))
            .height(Length::Px(CLOSE));
            children.push(close);
        }
        let painter = TabPainter {
            action: Action::new((self.map)(DockMsg(Inner::Activate(panel)))),
            active,
            palette: self.palette,
            title: title.clone(),
        };
        let end = Rc::clone(&self.map);
        Element::row(id.0, children)
            .gap(4.)
            .insets(Insets {
                left: 10.,
                right: 6.,
                ..Insets::default()
            })
            .align(Align::Center)
            .height(Length::Fill(1.))
            .painter(painter)
            .drag_payload(Payload::new(PAYLOAD, title, (self.id.0, panel)))
            .on_drag(move |e: &DragEvent| {
                (e.phase == DragPhase::End).then(|| end(DockMsg(Inner::DragEnd)))
            })
    }
}

/// La boîte de l'élément qui reçoit `event`, dans la fenêtre.
fn box_of(event: &DragEvent) -> RectF {
    let (left, top) = (event.x - event.local_x, event.y - event.local_y);
    RectF {
        left,
        top,
        right: left + event.width,
        bottom: top + event.height,
    }
}

/// L'onglet tiré, s'il vient de la zone `area`.
fn dragged(event: &DragEvent, area: u64) -> Option<u64> {
    if !event.payload.is(PAYLOAD) {
        return None;
    }
    let &(from, panel) = event.payload.get::<(u64, u64)>()?;
    (from == area).then_some(panel)
}

/// Côté d'un dépôt dans un panneau : le quart du bord le plus proche, ou le milieu.
pub(super) fn side_of(event: &DragEvent) -> DockSide {
    let fx = if event.width > 0. {
        event.local_x / event.width
    } else {
        0.5
    };
    let fy = if event.height > 0. {
        event.local_y / event.height
    } else {
        0.5
    };
    let edges = [
        (fx, DockSide::Left),
        (1. - fx, DockSide::Right),
        (fy, DockSide::Top),
        (1. - fy, DockSide::Bottom),
    ];
    let (distance, side) = edges
        .into_iter()
        .fold((f32::MAX, DockSide::Center), |best, edge| {
            if edge.0 < best.0 { edge } else { best }
        });
    if distance > 0.25 {
        DockSide::Center
    } else {
        side
    }
}

/// Fond d'un onglet : l'onglet montré a la couleur du panneau et un trait d'accent.
struct TabPainter {
    action: Action,
    active: bool,
    palette: EditorPalette,
    title: String,
}

impl Widget for TabPainter {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        let r = RectF {
            left: r.left - 10.,
            right: r.right + 6.,
            ..r
        };
        if self.active {
            scene.fill(r, 0., p.panel);
            scene.fill(
                RectF {
                    bottom: r.top + 2.,
                    ..r
                },
                0.,
                p.accent,
            );
        } else if context.state.hovered {
            scene.fill(r, 0., p.hover);
        }
        if context.state.focused {
            scene.commands.push(crate::ui::scene::DrawCommand::Stroke {
                rect: r,
                radius: 0.,
                width: 1.,
                color: p.accent,
            });
        }
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Tab,
            name: &self.title,
            enabled: true,
        }
    }
}

/// La croix qui ferme un onglet.
struct CloseButton {
    action: Action,
    color: Color,
}

impl Widget for CloseButton {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: CLOSE,
            height: CLOSE,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        if context.state.hovered {
            scene.fill(
                r,
                3.,
                Color {
                    alpha: 0x40,
                    ..self.color
                },
            );
        }
        let (cx, cy, h) = ((r.left + r.right) / 2., (r.top + r.bottom) / 2., 3.5);
        for (a, b) in [((-h, -h), (h, h)), ((-h, h), (h, -h))] {
            scene.commands.push(crate::ui::scene::DrawCommand::Line {
                start: crate::ui::scene::PointF {
                    x: cx + a.0,
                    y: cy + a.1,
                },
                end: crate::ui::scene::PointF {
                    x: cx + b.0,
                    y: cy + b.1,
                },
                width: 1.2,
                color: self.color,
            });
        }
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: "Fermer",
            enabled: true,
        }
    }
}

/// La marque du dépôt : la part du groupe que prendra l'onglet, teintée d'accent.
struct Mark {
    side: DockSide,
    color: Color,
}

impl Widget for Mark {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        let zone = match self.side {
            DockSide::Left => RectF {
                right: r.left + w / 2.,
                ..r
            },
            DockSide::Right => RectF {
                left: r.right - w / 2.,
                ..r
            },
            DockSide::Top => RectF {
                bottom: r.top + h / 2.,
                ..r
            },
            DockSide::Bottom => RectF {
                top: r.bottom - h / 2.,
                ..r
            },
            DockSide::Center => r,
        };
        scene.fill(
            zone,
            2.,
            Color {
                alpha: 0x38,
                ..self.color
            },
        );
        scene.commands.push(crate::ui::scene::DrawCommand::Stroke {
            rect: zone,
            radius: 2.,
            width: 2.,
            color: self.color,
        });
    }

    fn hit_test(&self, _: RectF, _: crate::ui::scene::PointF) -> bool {
        false
    }
}
