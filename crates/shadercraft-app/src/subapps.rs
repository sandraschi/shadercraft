//! Hierarchical `SubApp` sidebar registry for ShaderCraft's 5 workbenches.

use crate::app::{AppLocale, Page};

#[derive(Debug, Clone, Copy)]
pub struct SubPage {
    pub page: Page,
    pub sub: usize,
    pub en: &'static str,
    pub de: &'static str,
}

impl SubPage {
    pub fn label(&self, locale: AppLocale) -> &'static str {
        match locale {
            AppLocale::En => self.en,
            AppLocale::De => self.de,
        }
    }
}

pub trait SubApp: Sync {
    fn id(&self) -> &'static str;
    fn title(&self, locale: AppLocale) -> &'static str;
    fn subpages(&self) -> &'static [SubPage];
}

struct CraftWorkbench {
    id: &'static str,
    en: &'static str,
    de: &'static str,
    subpages: &'static [SubPage],
}

impl SubApp for CraftWorkbench {
    fn id(&self) -> &'static str {
        self.id
    }

    fn title(&self, locale: AppLocale) -> &'static str {
        match locale {
            AppLocale::En => self.en,
            AppLocale::De => self.de,
        }
    }

    fn subpages(&self) -> &'static [SubPage] {
        self.subpages
    }
}

const GRAPH_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::GraphEditor,
        sub: 0,
        en: "Node List & Parameters",
        de: "Knotenliste & Parameter",
    },
    SubPage {
        page: Page::GraphEditor,
        sub: 1,
        en: "Connections & New Nodes",
        de: "Verbindungen & Neue Knoten",
    },
];

const PREVIEW_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::Preview,
        sub: 0,
        en: "2D Tile Preview",
        de: "2D-Kachelvorschau",
    },
    SubPage {
        page: Page::Preview,
        sub: 1,
        en: "Statistics & Export",
        de: "Statistik & Export",
    },
];

const PBR_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::PbrMaps,
        sub: 0,
        en: "Derived Maps Grid",
        de: "Abgeleitete Karten",
    },
    SubPage {
        page: Page::PbrMaps,
        sub: 1,
        en: "Map Details",
        de: "Kartendetails",
    },
];

const PRESET_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::Presets,
        sub: 0,
        en: "Material Preset Gallery",
        de: "Materialvorlagen",
    },
    SubPage {
        page: Page::Presets,
        sub: 1,
        en: "Starter Graphs",
        de: "Startgraphen",
    },
];

const ASSISTANT_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::Chat,
        sub: 0,
        en: "Chat",
        de: "Chat",
    },
    SubPage {
        page: Page::Help,
        sub: 0,
        en: "Help (F1)",
        de: "Hilfe (F1)",
    },
];

static WORKBENCHES: [CraftWorkbench; 5] = [
    CraftWorkbench {
        id: "graph",
        en: "1. Graph Editor",
        de: "1. Grapheneditor",
        subpages: GRAPH_SUBS,
    },
    CraftWorkbench {
        id: "preview",
        en: "2. 2D Preview",
        de: "2. 2D-Vorschau",
        subpages: PREVIEW_SUBS,
    },
    CraftWorkbench {
        id: "pbrmaps",
        en: "3. PBR Maps",
        de: "3. PBR-Karten",
        subpages: PBR_SUBS,
    },
    CraftWorkbench {
        id: "presets",
        en: "4. Presets",
        de: "4. Vorlagen",
        subpages: PRESET_SUBS,
    },
    CraftWorkbench {
        id: "assistant",
        en: "5. Assistant & Help",
        de: "5. Assistent & Hilfe",
        subpages: ASSISTANT_SUBS,
    },
];

pub fn registry() -> Vec<&'static dyn SubApp> {
    WORKBENCHES.iter().map(|w| w as &dyn SubApp).collect()
}
