//! Ready-to-use styled components, written like HTML and customised with attributes:
//!
//! ```ignore
//! use karbon_ui::ui::kit::prelude::*;
//!
//! ui! {
//!     <Stack gap="lg" padding="xl">
//!         <Button primary icon="plus" onclick={Msg::Add}>Add</Button>
//!     </Stack>
//! }
//! ```
//!
//! Components are grouped by type in the submodules ([`typography`], [`forms`], [`buttons`],
//! [`animation`], [`pagination`]…) and all available from [`prelude`].
//!
//! Each component is built on the native controls of the engine: keyboard, focus and
//! accessibility come with them. Colours and radii come from [`KitTheme`] and follow the
//! light or dark colour scheme.
use crate::ui::template::Slots;

crate::css_file!(mod css, "css/kit.css");

/// Gabarit d'un composant du kit : feuille du kit, variables du thème.
macro_rules! kit_ui {
    ($cx:ident, $slots:ident; $($markup:tt)*) => {{
        let variables = super::theme::scope($cx.variables);
        let cx = $cx.with_variables(&variables);
        crate::ui! { @context(cx); @slots($slots); @styles(css); $($markup)* }
    }};
    ($cx:ident; $($markup:tt)*) => {{
        let variables = super::theme::scope($cx.variables);
        let cx = $cx.with_variables(&variables);
        crate::ui! { @context(cx); @styles(css); $($markup)* }
    }};
}

pub mod animation;
pub mod buttons;
pub mod carousel;
pub mod display;
pub mod effects;
mod events;
pub mod feedback;
pub mod forms;
pub mod icons;
pub mod layout;
pub mod navigation;
pub mod overlays;
pub mod pagination;
mod props;
mod space;
mod theme;
mod tone;
pub mod typography;

pub use animation::Animate;
pub use buttons::{Button, IconButton};
pub use carousel::Carousel;
pub use display::{
    Accordion, Avatar, AvatarGroup, Badge, Card, EmptyState, List, ListItem, Snippet, Stat, Table,
    TableCell, TableRow, User,
};
pub use effects::{Aurora, Glass};
pub(crate) use events::ValueAction;
pub use events::{OnChange, OnClick};
pub use feedback::{Alert, Progress, Skeleton, Spinner, Toast, Toasts};
pub use forms::{Checkbox, Field, Input, RadioGroup, Select, Slider, Switch, Textarea};
pub use icons::{ICON_NAMES, Icon};
pub use layout::{Container, Divider, Grid, Page, Row, Spacer, Stack};
pub use navigation::{Breadcrumb, NavItem, NavLabel, Navbar, Sidebar, Steps, Tabs};
pub use overlays::{Dialog, Dropdown, MenuItem, Tooltip};
pub use pagination::Pagination;
pub use props::{Number, Options, Text as TextValue, Texts};
pub use space::Space;
pub use theme::{KitTheme, Palette, set_theme, theme, variables};
pub(crate) use theme::{engine_theme, revision, take_used};
pub use tone::{CustomColor, Variant};
pub use typography::{Code, Kbd, Link, Text, Title};

/// Le slot `name` a du contenu.
fn has_slot(slots: &Slots, name: &str) -> bool {
    slots.unused_names().any(|used| used == name)
}
/// Texte du contenu du slot par défaut.
fn slot_text(slots: &mut Slots) -> String {
    slots
        .take("")
        .iter()
        .filter_map(|element| element.widget.as_ref())
        .map(|widget| widget.semantics().name.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Everything needed to write a view with the kit: the usual prelude, with the kit's
/// components in place of the widgets of the same name.
pub mod prelude {
    pub use super::{
        Accordion, Alert, Animate, Aurora, Avatar, AvatarGroup, Badge, Breadcrumb, Button, Card,
        Carousel, Checkbox, Code, Container, CustomColor, Dialog, Divider, Dropdown, EmptyState,
        Field, Glass, Grid, Icon, IconButton, Input, Kbd, KitTheme, Link, List, ListItem, MenuItem,
        NavItem, NavLabel, Navbar, Page, Pagination, Palette, Progress, RadioGroup, Row, Select,
        Sidebar, Skeleton, Slider, Snippet, Spacer, Spinner, Stack, Stat, Steps, Switch, Table,
        TableCell, TableRow, Tabs, Text, Textarea, Title, Toast, Toasts, Tooltip, User, Variant,
        set_theme,
    };
    pub use crate::ui::prelude::*;
}
