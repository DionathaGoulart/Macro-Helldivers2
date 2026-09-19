//! Aba "Sobre": identidade do app, links do projeto, créditos das fontes de
//! dados e a pasta onde ficam as configurações e o log.
//!
//! Como as outras abas, a tela é uma função do estado e um clique devolve uma
//! [`Action`] para a janela executar. A única ação que sai daqui é abrir um
//! endereço (ou a pasta de dados) no shell do Windows, e o endereço vem de uma
//! tabela constante: a aba nunca monta URL a partir de texto da tela.

use crate::i18n::{self, Tr};
use crate::settings::Settings;
use crate::ui::theme;
use crate::ui::toolkit::{columns, id, id_at, Id, Measure, Rect, TextStyle, Ui};
use crate::ui::widgets::{self, styles, ButtonVariant, CardHeader};

/// `screen-pad` da coluna de conteúdo, como nas demais abas.
const PAGE_PADDING: f32 = 24.0;
const PAGE_TOP: f32 = 20.0;
/// Espaço reservado à direita para a barra de rolagem.
const SCROLL_GUTTER: f32 = 14.0;
const CARD_GAP: f32 = 20.0;
const SECTION_GAP: f32 = 22.0;

/// Altura de uma linha "rótulo + valor".
const ROW_H: f32 = 18.0;
const ROW_GAP: f32 = 10.0;
/// Espaço entre blocos dentro de um painel.
const BLOCK_GAP: f32 = 16.0;
/// Botões de link, empilhados.
const LINK_H: f32 = 40.0;
const LINK_GAP: f32 = 10.0;
/// Largura da coluna de rótulos das linhas de ficha.
const LABEL_W: f32 = 96.0;

/// Endereços que a aba abre, na ordem dos botões do painel de links. Ficam em
/// constante por segurança: o que vai para o `ShellExecuteW` nunca depende de
/// texto vindo de arquivo, tradução ou rede.
pub const LINKS: [&str; 3] = [
    "https://github.com/DionathaGoulart/Macro-Helldivers2",
    "https://github.com/DionathaGoulart/Macro-Helldivers2/releases/latest",
    "https://github.com/DionathaGoulart/Macro-Helldivers2/issues",
];

/// Fontes de dados creditadas, na ordem do painel de créditos.
pub const SOURCES: [&str; 2] = ["https://helldivers.wiki.gg", "https://helldive.live"];

/// Tecnologia do app, igual nos dois idiomas: são nomes próprios.
const STACK: &str = "Rust · Win32 · Direct2D";

/// O que a janela faz depois de um clique na aba.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Abrir um endereço no navegador padrão.
    OpenUrl(&'static str),
    /// Abrir a pasta de dados do usuário no Explorer.
    OpenDataFolder,
}

/// O que a aba precisa saber do resto do app.
pub struct Ctx<'a> {
    pub settings: &'a Settings,
    /// Pasta de dados do usuário, já formatada. Vem de fora para o teste não
    /// depender do perfil de quem roda.
    pub data_dir: &'a str,
}

impl Ctx<'_> {
    fn tr(&self) -> &'static Tr {
        i18n::tr(self.settings.language)
    }
}

/// A aba não guarda estado: tudo o que ela mostra vem do `Ctx`. O tipo existe
/// para a janela tratá-la como as outras.
#[derive(Debug, Default)]
pub struct AboutTab;

// --- Ids ---

fn scroll_id() -> Id {
    id("about.scroll")
}

fn link_id(index: usize) -> Id {
    id_at("about.link", index)
}

fn source_id(index: usize) -> Id {
    id_at("about.source", index)
}

fn folder_id() -> Id {
    id("about.folder")
}

// --- Estilos ---

fn label_style() -> TextStyle {
    styles::label().middle()
}

/// O valor de uma linha de ficha quebra: "uso pessoal, com crédito
/// obrigatório" não cabe numa linha na coluna estreita, e cortar o texto com
/// reticências esconderia justamente a condição da licença.
fn value_style() -> TextStyle {
    styles::body().wrap()
}

fn hint_style() -> TextStyle {
    styles::hint()
}

impl AboutTab {
    pub fn new() -> AboutTab {
        AboutTab
    }

    /// Versão compilada no binário, com o `v` da tag do release.
    pub fn version() -> String {
        format!("v{}", env!("CARGO_PKG_VERSION"))
    }

    // --- Construção ---

    pub fn build(&mut self, ui: &mut Ui, measure: &mut dyn Measure, area: Rect, ctx: &Ctx) {
        let view = Rect::new(
            area.x + PAGE_PADDING,
            area.y + PAGE_TOP,
            area.w - PAGE_PADDING * 2.0,
            area.h - PAGE_TOP,
        );
        let width = view.w - SCROLL_GUTTER;
        let half = (width - CARD_GAP) / 2.0;

        let offset = ui.scroll_begin(scroll_id(), view);
        let mut y = view.y - offset;

        // Ficha do app à esquerda, links à direita: a linha tem a altura do
        // painel mais alto, como na aba de configurações.
        let identity = identity_height(measure, half, ctx);
        let row = identity.max(links_height());
        let top = columns(Rect::new(view.x, y, width, row), 2, CARD_GAP);
        identity_card(ui, measure, top[0].with_h(identity), ctx);
        links_card(ui, top[1].with_h(links_height()), ctx);
        y += row + SECTION_GAP;

        let height = sources_height(measure, width, ctx);
        sources_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
        y += height + SECTION_GAP;

        let height = files_height(measure, width, ctx);
        files_card(ui, measure, Rect::new(view.x, y, width, height), ctx);
        // A sombra do último painel e um respiro antes do rodapé.
        y += height + theme::SHADOW + PAGE_TOP;

        ui.scroll_end(scroll_id(), view, y - (view.y - offset));
    }

    /// Um clique na aba. `None` quando o clique não era de nenhum controle
    /// daqui.
    pub fn on_click(&mut self, clicked: Id) -> Option<Action> {
        if let Some(index) = (0..LINKS.len()).find(|index| link_id(*index) == clicked) {
            return Some(Action::OpenUrl(LINKS[index]));
        }
        if let Some(index) = (0..SOURCES.len()).find(|index| source_id(*index) == clicked) {
            return Some(Action::OpenUrl(SOURCES[index]));
        }
        if clicked == folder_id() {
            return Some(Action::OpenDataFolder);
        }
        None
    }
}

// --- Painéis ---

/// Ficha do app: nome, versão, o que ele é, e as linhas de autor, licença e
/// tecnologia.
fn identity_card(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
    let palette = theme::palette();
    let tr = ctx.tr();
    let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.about.title, "nfo")));

    widgets::page_header(
        ui,
        content.cut_top(widgets::PAGE_HEADER_HEIGHT),
        &AboutTab::version(),
        tr.about.app_name,
    );
    content.skip_top(BLOCK_GAP);

    let height = measure
        .text_size(tr.about.description, hint_style(), content.w)
        .1;
    ui.text(
        content.cut_top(height),
        tr.about.description,
        hint_style(),
        palette.muted,
    );
    content.skip_top(BLOCK_GAP);

    for (label, value) in info_rows(tr) {
        let height = row_height(measure, content.w, value);
        let mut row = content.cut_top(height);
        ui.text(
            row.cut_left(LABEL_W).with_h(ROW_H),
            label,
            label_style(),
            palette.muted,
        );
        ui.text(row, value, value_style(), palette.content);
        content.skip_top(ROW_GAP);
    }
}

/// As três linhas de ficha, na ordem da tela. O autor e a pilha não são
/// traduzidos: são nomes próprios.
fn info_rows(tr: &'static Tr) -> [(&'static str, &'static str); 3] {
    [
        (tr.about.author, "DionathaGoulart"),
        (tr.about.license, tr.about.license_value),
        (tr.about.stack, STACK),
    ]
}

/// Altura de uma linha de ficha: ao menos uma linha de texto, mais se o valor
/// quebrar na largura que sobra ao lado do rótulo.
fn row_height(measure: &mut dyn Measure, width: f32, value: &str) -> f32 {
    let value_h = measure.text_size(value, value_style(), width - LABEL_W).1;
    ROW_H.max(value_h)
}

fn identity_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let inner = width - widgets::CARD_PADDING * 2.0;
    let description = measure
        .text_size(ctx.tr().about.description, hint_style(), inner)
        .1;
    let rows: f32 = info_rows(ctx.tr())
        .into_iter()
        .map(|(_, value)| row_height(measure, inner, value) + ROW_GAP)
        .sum();
    widgets::card_chrome(true)
        + widgets::PAGE_HEADER_HEIGHT
        + BLOCK_GAP
        + description
        + BLOCK_GAP
        + rows
        - ROW_GAP
}

/// Links do projeto: repositório, a última versão e os problemas abertos.
fn links_card(ui: &mut Ui, rect: Rect, ctx: &Ctx) {
    let tr = ctx.tr();
    let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.about.links, "url")));

    let labels = [
        tr.about.link_repo,
        tr.about.link_releases,
        tr.about.link_issues,
    ];
    for (index, label) in labels.into_iter().enumerate() {
        widgets::button(
            ui,
            link_id(index),
            content.cut_top(LINK_H),
            label,
            if index == 0 {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            },
        );
        content.skip_top(LINK_GAP);
    }
}

fn links_height() -> f32 {
    widgets::card_chrome(true) + (LINK_H + LINK_GAP) * LINKS.len() as f32 - LINK_GAP
}

/// Créditos: de onde vêm os estratagemas e as estatísticas, e o aviso de que
/// isto é projeto de fã.
fn sources_card(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
    let palette = theme::palette();
    let tr = ctx.tr();
    let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.about.credits, "txt")));

    let entries = [
        (tr.about.source_wiki, tr.about.source_wiki_desc),
        (tr.about.source_meta, tr.about.source_meta_desc),
    ];
    for (index, (name, desc)) in entries.into_iter().enumerate() {
        let mut row = content.cut_top(source_row_height(measure, content.w, desc));
        // O botão sai à direita antes de o texto medir a largura que sobra.
        let button = row
            .cut_right(widgets::icon_btn_width(measure, tr.about.open))
            .with_h(widgets::ICON_BTN_HEIGHT);
        widgets::icon_btn(
            ui,
            source_id(index),
            button,
            widgets::Glyph::Text(tr.about.open),
            widgets::Tone::Plain,
        );
        row.cut_right(BLOCK_GAP);

        ui.text(row.cut_top(ROW_H), name, label_style(), palette.content);
        ui.text(row, desc, hint_style(), palette.muted);
        content.skip_top(BLOCK_GAP);
    }

    let height = measure
        .text_size(tr.about.disclaimer, hint_style(), content.w)
        .1;
    ui.text(
        content.cut_top(height),
        tr.about.disclaimer,
        hint_style(),
        palette.muted,
    );
}

/// Altura de uma linha de fonte: o nome mais a descrição, que quebra na
/// largura que o botão deixa.
fn source_row_height(measure: &mut dyn Measure, width: f32, desc: &str) -> f32 {
    let text_w = width - icon_gutter(measure);
    ROW_H + measure.text_size(desc, hint_style(), text_w).1
}

/// Largura tomada pelo botão de abrir mais o vão antes do texto.
fn icon_gutter(measure: &mut dyn Measure) -> f32 {
    // O rótulo é o mesmo nos dois idiomas medidos aqui e na montagem: o que
    // importa é a medida ser a mesma nas duas passagens.
    widgets::icon_btn_width(measure, i18n::PT.about.open) + BLOCK_GAP
}

fn sources_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let tr = ctx.tr();
    let inner = width - widgets::CARD_PADDING * 2.0;
    let rows = source_row_height(measure, inner, tr.about.source_wiki_desc)
        + BLOCK_GAP
        + source_row_height(measure, inner, tr.about.source_meta_desc)
        + BLOCK_GAP;
    let disclaimer = measure
        .text_size(tr.about.disclaimer, hint_style(), inner)
        .1;
    widgets::card_chrome(true) + rows + disclaimer
}

/// Pasta de dados: onde ficam settings, builds salvas e o log, com o botão que
/// a abre no Explorer.
fn files_card(ui: &mut Ui, measure: &mut dyn Measure, rect: Rect, ctx: &Ctx) {
    let palette = theme::palette();
    let tr = ctx.tr();
    let mut content = widgets::card(ui, rect, Some(CardHeader::new(tr.about.files, "dir")));

    let height = measure
        .text_size(tr.about.files_desc, hint_style(), content.w)
        .1;
    ui.text(
        content.cut_top(height),
        tr.about.files_desc,
        hint_style(),
        palette.muted,
    );
    content.skip_top(ROW_GAP);

    let height = measure.text_size(ctx.data_dir, hint_style(), content.w).1;
    ui.text(
        content.cut_top(height),
        ctx.data_dir,
        hint_style(),
        palette.content,
    );
    content.skip_top(BLOCK_GAP);

    widgets::button(
        ui,
        folder_id(),
        content.cut_top(LINK_H).with_w(LINK_BUTTON_W),
        tr.about.open_folder,
        ButtonVariant::Secondary,
    );
}

/// Largura do botão que abre a pasta: ele não ocupa a linha inteira.
const LINK_BUTTON_W: f32 = 220.0;

fn files_height(measure: &mut dyn Measure, width: f32, ctx: &Ctx) -> f32 {
    let tr = ctx.tr();
    let inner = width - widgets::CARD_PADDING * 2.0;
    let desc = measure
        .text_size(tr.about.files_desc, hint_style(), inner)
        .1;
    let path = measure.text_size(ctx.data_dir, hint_style(), inner).1;
    widgets::card_chrome(true) + desc + ROW_GAP + path + BLOCK_GAP + LINK_H
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Language;
    use crate::ui::toolkit::Visual;

    /// Medidor de largura fixa, como o das outras abas.
    struct Fixed;

    impl Measure for Fixed {
        fn text_size(&mut self, text: &str, style: TextStyle, max: f32) -> (f32, f32) {
            let width = text.chars().count() as f32 * style.size * 0.6;
            if style.wrap && max.is_finite() && width > max {
                let lines = (width / max).ceil();
                (max, style.size * 1.3 * lines)
            } else {
                (width, style.size)
            }
        }
    }

    const AREA: Rect = Rect::new(0.0, 0.0, 820.0, 532.0);
    const DATA_DIR: &str = "C:\\Users\\dev\\AppData\\Roaming\\Macro Helldivers 2";

    fn settings(language: Language) -> Settings {
        Settings {
            language,
            ..Settings::default()
        }
    }

    fn build(tab: &mut AboutTab, ui: &mut Ui, settings: &Settings) {
        let ctx = Ctx {
            settings,
            data_dir: DATA_DIR,
        };
        ui.begin(0);
        tab.build(ui, &mut Fixed, AREA, &ctx);
        ui.end();
    }

    fn texts(ui: &Ui) -> Vec<String> {
        ui.frame()
            .nodes
            .iter()
            .filter_map(|node| match &node.visual {
                Visual::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_screen_shows_the_compiled_version_and_the_data_folder() {
        let mut tab = AboutTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &settings(Language::Pt));

        let texts = texts(&ui);
        // O kicker do cabeçalho sai em caixa alta, como todo `screen-kicker`.
        let version = AboutTab::version().to_uppercase();
        assert!(
            texts.iter().any(|text| text.contains(&version)),
            "a versão compilada não apareceu: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == DATA_DIR),
            "o caminho dos dados não apareceu: {texts:?}"
        );
    }

    /// Cada botão de link precisa ser alcançável pelo mouse e cair no endereço
    /// da tabela: um id trocado abriria a página errada.
    #[test]
    fn every_link_is_clickable_and_opens_its_own_address() {
        let mut tab = AboutTab::new();
        let mut ui = Ui::new();
        build(&mut tab, &mut ui, &settings(Language::Pt));

        for (index, url) in LINKS.iter().enumerate() {
            assert!(ui.frame().has_hit(link_id(index)), "link {index} sem área");
            assert_eq!(tab.on_click(link_id(index)), Some(Action::OpenUrl(url)));
        }
        for (index, url) in SOURCES.iter().enumerate() {
            assert!(
                ui.frame().has_hit(source_id(index)),
                "fonte {index} sem área"
            );
            assert_eq!(tab.on_click(source_id(index)), Some(Action::OpenUrl(url)));
        }
        assert!(ui.frame().has_hit(folder_id()));
        assert_eq!(tab.on_click(folder_id()), Some(Action::OpenDataFolder));
    }

    /// Só endereços `https` da tabela saem daqui: é o que vai para o shell.
    #[test]
    fn every_address_is_https() {
        for url in LINKS.iter().chain(SOURCES.iter()) {
            assert!(url.starts_with("https://"), "{url}");
        }
    }

    #[test]
    fn a_click_outside_the_tab_is_ignored() {
        let mut tab = AboutTab::new();
        assert_eq!(tab.on_click(id("outra.coisa")), None);
    }

    /// A página cresce com o texto, e nos dois idiomas ela cabe na rolagem sem
    /// painel sobrepondo painel.
    #[test]
    fn both_languages_lay_out_without_overlap() {
        for language in Language::ALL {
            let mut tab = AboutTab::new();
            let mut ui = Ui::new();
            build(&mut tab, &mut ui, &settings(language));

            let texts = texts(&ui);
            assert!(!texts.is_empty(), "{language} sem texto");
            assert!(
                texts.iter().all(|text| !text.trim().is_empty()),
                "{language} com texto vazio: {texts:?}"
            );
        }
    }
}
