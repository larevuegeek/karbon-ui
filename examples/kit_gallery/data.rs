//! Données fictives des écrans d'exemple, et extraits de code affichés dans la galerie.
use karbon_ui::ui::prelude::{Color, LinearGradient, linear_gradient};

/// Rubriques : nom, icône, description.
pub const SECTIONS: [(&str, &str, &str); 19] = [
    ("Aperçu", "home", ""),
    (
        "Tableau de bord",
        "star",
        "Chiffres, graphique et commandes récentes.",
    ),
    (
        "Boutique",
        "image",
        "Catalogue filtrable, favoris et panier.",
    ),
    ("Lecteur", "music", "Lecture, file d'attente et favoris."),
    (
        "Tarifs",
        "check-circle",
        "Offres, facturation mensuelle ou annuelle.",
    ),
    ("Connexion", "lock", "Un formulaire qui valide vraiment."),
    (
        "Réglages",
        "settings",
        "Profil, notifications et apparence.",
    ),
    ("Messagerie", "mail", "Conversations et envoi de messages."),
    (
        "Typographie",
        "edit",
        "Titres, paragraphes, code et icônes.",
    ),
    ("Boutons", "plus", "Couleurs, apparences, tailles et états."),
    (
        "Formulaires",
        "file",
        "Champs, listes, cases, interrupteurs et curseurs.",
    ),
    (
        "Données",
        "folder",
        "Chiffres clés, tableaux, listes et accordéons.",
    ),
    (
        "Retours d'état",
        "bell",
        "Alertes, progression et chargement.",
    ),
    ("Navigation", "menu", "Onglets, fil d'Ariane et étapes."),
    (
        "Pagination",
        "more-horizontal",
        "Parcourir une longue liste, page par page.",
    ),
    ("Carrousel", "image", "Faire défiler des diapositives."),
    (
        "Animation",
        "play",
        "Entrées et boucles, par simple attribut.",
    ),
    (
        "Événements",
        "info",
        "Survol, appui, focus, clics : tout arrive dans update.",
    ),
    ("Couches", "copy", "Dialogues, menus et infobulles."),
];
/// Première rubrique de composants ; les précédentes sont des écrans d'exemple.
pub const FIRST_COMPONENT: usize = 8;
pub const EVENTS_SECTION: usize = 17;

pub const ACCENTS: [u32; 4] = [0x006fee, 0x7828c8, 0xf31260, 0x12a150];

/// Dégradé de la couverture `index`.
pub fn cover(index: usize) -> LinearGradient {
    const PAIRS: [(u32, u32); 8] = [
        (0x006fee, 0x7828c8),
        (0xf31260, 0xf5a524),
        (0x12a150, 0x0ea5e9),
        (0x7828c8, 0xf31260),
        (0x0ea5e9, 0x006fee),
        (0xf5a524, 0xf31260),
        (0x3f3f46, 0x18181b),
        (0x17c964, 0x006fee),
    ];
    let (a, b) = PAIRS[index % PAIRS.len()];
    linear_gradient(45., [Color::hex(a), Color::hex(b)])
}

pub struct Range {
    /// Libellé, valeur, évolution, en hausse, icône.
    pub stats: [(&'static str, &'static str, &'static str, bool, &'static str); 4],
    pub bars: [f32; 12],
}
pub const RANGES: [Range; 3] = [
    Range {
        stats: [
            ("Revenus", "12 480 €", "+8,2 %", true, "star"),
            ("Abonnés", "1 284", "+64", true, "users"),
            ("Écoutes", "48,1 k", "+12,4 %", true, "music"),
            ("Résiliations", "11", "+3", false, "log-out"),
        ],
        bars: [34., 52., 41., 68., 57., 80., 72., 46., 63., 88., 76., 94.],
    },
    Range {
        stats: [
            ("Revenus", "52 910 €", "+14,0 %", true, "star"),
            ("Abonnés", "5 402", "+312", true, "users"),
            ("Écoutes", "211 k", "+6,1 %", true, "music"),
            ("Résiliations", "38", "-9", true, "log-out"),
        ],
        bars: [62., 48., 71., 55., 83., 67., 90., 74., 58., 79., 95., 86.],
    },
    Range {
        stats: [
            ("Revenus", "604 k€", "+31,5 %", true, "star"),
            ("Abonnés", "41 870", "+9 120", true, "users"),
            ("Écoutes", "2,4 M", "+22,8 %", true, "music"),
            ("Résiliations", "412", "+27", false, "log-out"),
        ],
        bars: [28., 35., 42., 39., 51., 58., 64., 61., 72., 78., 87., 96.],
    },
];
pub const MONTHS: [&str; 12] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];
pub const STATUS: [&str; 3] = ["Payée", "En attente", "Échouée"];
/// Client, offre, montant, état.
pub const ORDERS: [(&str, &str, &str, usize); 4] = [
    ("Ada Lovelace", "Offre Pro", "29,00 €", 0),
    ("Alan Turing", "Offre Équipe", "99,00 €", 0),
    ("Grace Hopper", "Offre Pro", "29,00 €", 1),
    ("Linus Torvalds", "Offre Découverte", "9,00 €", 2),
];

pub const CATEGORIES: [&str; 4] = ["Tout", "Audio", "Vinyles", "Accessoires"];
/// Nom, catégorie, prix en euros, note.
pub type Product = (&'static str, usize, u32, &'static str);
pub const PRODUCTS: [Product; 8] = [
    ("Casque Studio", 1, 249, "4,8"),
    ("Enceinte Nomade", 1, 129, "4,6"),
    ("Blue Train, édition mono", 2, 34, "4,9"),
    ("Kind of Blue", 2, 29, "5,0"),
    ("Platine Compacte", 1, 399, "4,7"),
    ("Brosse antistatique", 3, 12, "4,4"),
    ("Câble tressé", 3, 19, "4,5"),
    ("Pochettes intérieures", 3, 15, "4,3"),
];

/// Titre, artiste, durée en secondes.
pub const TRACKS: [(&str, &str, u32); 5] = [
    ("Blue Train", "John Coltrane", 643),
    ("So What", "Miles Davis", 562),
    ("Take Five", "Dave Brubeck", 324),
    ("Cantaloupe Island", "Herbie Hancock", 330),
    ("Moanin'", "Art Blakey", 575),
];
pub fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Nom, description, prix mensuel, prix annuel (par mois), avantages.
pub const PLANS: [(&str, &str, u32, u32, [&str; 4]); 3] = [
    (
        "Découverte",
        "Pour essayer, sans engagement.",
        9,
        7,
        [
            "1 utilisateur",
            "10 projets",
            "Thème clair et sombre",
            "Assistance par courriel",
        ],
    ),
    (
        "Pro",
        "Pour les indépendants exigeants.",
        29,
        24,
        [
            "5 utilisateurs",
            "Projets illimités",
            "Composants avancés",
            "Assistance prioritaire",
        ],
    ),
    (
        "Équipe",
        "Pour les studios et les entreprises.",
        99,
        79,
        [
            "Utilisateurs illimités",
            "Projets illimités",
            "Accès anticipé",
            "Interlocuteur dédié",
        ],
    ),
];

pub const SETTINGS_TABS: [(&str, &str); 3] = [
    ("Profil", "user"),
    ("Notifications", "bell"),
    ("Apparence", "sun"),
];

/// Nom, dernier message, heure, non lus.
pub const CONVERSATIONS: [(&str, &str, &str, u32); 4] = [
    ("Ada Lovelace", "Le calcul est terminé.", "14:02", 2),
    ("Alan Turing", "On se voit à Bletchley ?", "12:40", 0),
    ("Grace Hopper", "J'ai trouvé le bogue.", "Hier", 1),
    ("Linus Torvalds", "Envoyez le correctif.", "Lun.", 0),
];

pub const CODE_BUTTONS: &str = r#"<Button primary>Primaire</Button>
<Button primary soft icon="plus" onclick={Msg::Add}>Ajouter</Button>
<Button danger shadow pill lg>Supprimer</Button>"#;
pub const CODE_FORMS: &str = r#"<Input label="Courriel" icon="mail" value={&self.mail}
       oninput={Msg::Mail} error={self.mail_error()} />
<Select label="Genre" value={self.genre}
        options={["Jazz", "Rock"]} onchange={Msg::Genre} />
<Switch label="Notifications" checked={self.notify} onchange={Msg::Notify} />"#;
pub const CODE_CAROUSEL: &str = r#"<Carousel value={self.slide} onchange={Msg::Slide} per_view={3} loop_around>
    <Card title="Un" />
    <Card title="Deux" />
</Carousel>"#;
pub const CODE_ANIMATION: &str = r#"<Animate up delay={80}><Card>Entrée</Card></Animate>
<Animate float><Avatar name="Ada" /></Animate>"#;
pub const CODE_EVENTS: &str = r#"<Button onclick={Msg::Save}
        onhover={Msg::Hover}
        onpress={|down: bool| Msg::Press(down)}
        onfocus={Msg::Focus}
        ondblclick={Msg::Open}>Enregistrer</Button>"#;
pub const CODE_THEME: &str =
    r#"set_theme(KitTheme::default().primary(Color::hex(0xe11d48)).radius(16.));"#;
