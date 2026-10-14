//! `ui.Paragraph`, `ui.List`, `ui.Table` / `ui.Row`, `ui.Gauge` and
//! `ui.Border`: builder userdata (every method returns the widget, so
//! calls chain). They expose no getters, so a widget built by the host
//! from a file the plugin may not read (`pairee.preview_code`) cannot be
//! read back by the plugin.
//!
//! The 1.0 call forms keep working: `ui.Gauge(ratio, label)`,
//! `ui.List({"a", "b"})`, `ui.Table(headers, rows)`.

use super::budget::Budget;
use super::layout::constraint_list;
use super::style::{add_style_methods, style_arg};
use super::text::{align_arg, line_arg, span_arg, text_arg};
use crate::app::state::popup::plugin_widget::{
    WBorder, WBorderType, WGauge, WList, WParagraph, WRow, WStyle, WTable, WText, WWrap, edge,
};
use mlua::{AnyUserData, Lua, Table, UserData, UserDataMethods, Value};

/// `ui.Border` / `ui.Edge` / `ui.Wrap` names.
fn wrap_arg(name: &str) -> mlua::Result<WWrap> {
    match name {
        "no" => Ok(WWrap::No),
        "yes" => Ok(WWrap::Yes),
        "trim" => Ok(WWrap::Trim),
        other => Err(mlua::Error::RuntimeError(format!(
            "unknown wrap {other:?} (use ui.Wrap)"
        ))),
    }
}

fn border_type_arg(name: &str) -> mlua::Result<WBorderType> {
    match name {
        "plain" => Ok(WBorderType::Plain),
        "rounded" => Ok(WBorderType::Rounded),
        "double" => Ok(WBorderType::Double),
        "thick" => Ok(WBorderType::Thick),
        other => Err(mlua::Error::RuntimeError(format!(
            "unknown border type {other:?} (use ui.Border.*)"
        ))),
    }
}

fn border_arg(value: &Value) -> mlua::Result<WBorder> {
    match value {
        Value::UserData(ud) => Ok(ud.borrow::<WBorder>()?.clone()),
        other => Err(mlua::Error::RuntimeError(format!(
            "expected a ui.Border, got {}",
            other.type_name()
        ))),
    }
}

/// A widget that can be framed with `:block(ui.Border(...))`.
trait Framed {
    fn block_mut(&mut self) -> &mut Option<WBorder>;
}

macro_rules! framed {
    ($($ty:ty),*) => {$(
        impl Framed for $ty {
            fn block_mut(&mut self) -> &mut Option<WBorder> {
                &mut self.block
            }
        }
    )*};
}
framed!(WParagraph, WList, WTable, WGauge);

/// `:block(border)` plus `:area(rect)`, shared by the framed widgets.
fn add_block_method<T: Framed + 'static, M: UserDataMethods<T>>(methods: &mut M) {
    methods.add_function("block", |_, (ud, border): (AnyUserData, Value)| {
        *ud.borrow_mut::<T>()?.block_mut() = Some(border_arg(&border)?);
        Ok(ud)
    });
    super::geometry::add_area_method(methods);
}

impl UserData for WParagraph {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        add_block_method(methods);
        methods.add_function("align", |_, (ud, align): (AnyUserData, String)| {
            ud.borrow_mut::<WParagraph>()?.text.align = Some(align_arg(&align)?);
            Ok(ud)
        });
        methods.add_function("wrap", |_, (ud, wrap): (AnyUserData, String)| {
            ud.borrow_mut::<WParagraph>()?.wrap = wrap_arg(&wrap)?;
            Ok(ud)
        });
        methods.add_function("scroll", |_, (ud, lines): (AnyUserData, u16)| {
            ud.borrow_mut::<WParagraph>()?.scroll = lines;
            Ok(ud)
        });
    }
}

impl UserData for WList {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        add_block_method(methods);
    }
}

impl UserData for WRow {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        methods.add_function("height", |_, (ud, height): (AnyUserData, u16)| {
            ud.borrow_mut::<WRow>()?.height = height.max(1);
            Ok(ud)
        });
    }
}

impl UserData for WTable {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        add_block_method(methods);
        methods.add_function("header", |_, (ud, row): (AnyUserData, Value)| {
            ud.borrow_mut::<WTable>()?.header = Some(row_arg(&row)?);
            Ok(ud)
        });
        methods.add_function("widths", |_, (ud, widths): (AnyUserData, Table)| {
            ud.borrow_mut::<WTable>()?.widths = constraint_list(&widths)?;
            Ok(ud)
        });
        methods.add_function("spacing", |_, (ud, spacing): (AnyUserData, u16)| {
            ud.borrow_mut::<WTable>()?.spacing = spacing;
            Ok(ud)
        });
    }
}

impl UserData for WGauge {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        add_block_method(methods);
        methods.add_function("ratio", |_, (ud, ratio): (AnyUserData, f64)| {
            ud.borrow_mut::<WGauge>()?.ratio = clamp_ratio(ratio);
            Ok(ud)
        });
        methods.add_function("percent", |_, (ud, percent): (AnyUserData, f64)| {
            ud.borrow_mut::<WGauge>()?.ratio = clamp_ratio(percent / 100.0);
            Ok(ud)
        });
        methods.add_function("label", |_, (ud, label): (AnyUserData, Value)| {
            ud.borrow_mut::<WGauge>()?.label = Some(span_arg(&label)?);
            Ok(ud)
        });
        methods.add_function("gauge_style", |_, (ud, style): (AnyUserData, Value)| {
            ud.borrow_mut::<WGauge>()?.gauge_style = style_arg(&style)?;
            Ok(ud)
        });
    }
}

impl UserData for WBorder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        add_style_methods(methods);
        methods.add_function("type", |_, (ud, kind): (AnyUserData, String)| {
            ud.borrow_mut::<WBorder>()?.kind = border_type_arg(&kind)?;
            Ok(ud)
        });
        methods.add_function("title", |_, (ud, title): (AnyUserData, Value)| {
            ud.borrow_mut::<WBorder>()?.title = Some(line_arg(&title)?);
            Ok(ud)
        });
    }
}

/// A gauge ratio in `0..=1` (`0` for NaN and infinities, which the
/// renderer would reject).
pub(super) fn clamp_ratio(ratio: f64) -> f64 {
    if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// A `ui.Row` or a list of cells.
fn row_arg(value: &Value) -> mlua::Result<WRow> {
    match value {
        Value::UserData(ud) => Ok(ud.borrow::<WRow>()?.clone()),
        Value::Table(cells) => Ok(WRow {
            cells: texts_arg(cells)?,
            height: 1,
            style: WStyle::default(),
        }),
        other => Err(mlua::Error::RuntimeError(format!(
            "expected a ui.Row or a list of cells, got {}",
            other.type_name()
        ))),
    }
}

fn rows_arg(rows: &Table) -> mlua::Result<Vec<WRow>> {
    let mut budget = Budget::default();
    rows.sequence_values::<Value>()
        .map(|row| {
            let row = row_arg(&row?)?;
            budget.charge(1, 0)?;
            row.cells.iter().try_for_each(|cell| budget.text(cell))?;
            Ok(row)
        })
        .collect()
}

/// Every item of a list as text.
fn texts_arg(items: &Table) -> mlua::Result<Vec<WText>> {
    let mut budget = Budget::default();
    items
        .sequence_values::<Value>()
        .map(|item| {
            let text = text_arg(&item?)?;
            budget.text(&text)?;
            Ok(text)
        })
        .collect()
}

fn paragraph(text: WText) -> WParagraph {
    WParagraph {
        text,
        wrap: WWrap::No,
        scroll: 0,
        block: None,
        style: WStyle::default(),
    }
}

/// Adds `Paragraph`, `List`, `Row`, `Table` and `Gauge` to `ui`.
pub fn bind(lua: &Lua, ui: &Table) -> mlua::Result<()> {
    ui.set(
        "Paragraph",
        lua.create_function(|_, text: Value| Ok(paragraph(text_arg(&text)?)))?,
    )?;
    ui.set(
        "List",
        lua.create_function(|_, items: Table| {
            Ok(WList {
                items: texts_arg(&items)?,
                block: None,
                style: WStyle::default(),
            })
        })?,
    )?;
    ui.set(
        "Row",
        lua.create_function(|_, cells: Value| row_arg(&cells))?,
    )?;
    ui.set(
        "Table",
        lua.create_function(|_, (first, rows): (Table, Option<Table>)| {
            // 1.0 form: Table(headers, rows).
            let (header, rows) = match rows {
                Some(rows) => (Some(row_arg(&Value::Table(first))?), rows_arg(&rows)?),
                None => (None, rows_arg(&first)?),
            };
            Ok(WTable {
                header,
                rows,
                widths: Vec::new(),
                spacing: 1,
                block: None,
                style: WStyle::default(),
            })
        })?,
    )?;
    ui.set(
        "Gauge",
        lua.create_function(|_, (ratio, label): (Option<f64>, Value)| {
            Ok(WGauge {
                ratio: clamp_ratio(ratio.unwrap_or(0.0)),
                label: (!label.is_nil()).then(|| span_arg(&label)).transpose()?,
                block: None,
                style: WStyle::default(),
                gauge_style: WStyle::default(),
            })
        })?,
    )?;
    bind_border(lua, ui)
}

/// `ui.Border(edges?)` with the `ui.Border.*` types, and `ui.Edge`.
fn bind_border(lua: &Lua, ui: &Table) -> mlua::Result<()> {
    let make = lua.create_function(|_, edges: Option<u8>| {
        Ok(WBorder {
            edges: edges.unwrap_or(edge::ALL) & edge::ALL,
            kind: WBorderType::Plain,
            title: None,
            style: WStyle::default(),
        })
    })?;
    let border = super::text::callable(lua, make)?;
    for name in ["plain", "rounded", "double", "thick"] {
        border.set(name.to_ascii_uppercase(), name)?;
    }
    ui.set("Border", border)?;
    let edges = lua.create_table()?;
    for (name, bits) in [
        ("NONE", 0),
        ("TOP", edge::TOP),
        ("RIGHT", edge::RIGHT),
        ("BOTTOM", edge::BOTTOM),
        ("LEFT", edge::LEFT),
        ("ALL", edge::ALL),
    ] {
        edges.set(name, bits)?;
    }
    ui.set("Edge", edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::plugin_widget::WTextBody;

    fn lua() -> Lua {
        let lua = Lua::new();
        let ui = lua.create_table().unwrap();
        super::super::text::bind(&lua, &ui).unwrap();
        super::super::layout::bind(&lua, &ui).unwrap();
        bind(&lua, &ui).unwrap();
        lua.globals().set("ui", ui).unwrap();
        lua
    }

    fn get<T: Clone + 'static>(lua: &Lua, code: &str) -> T {
        let ud: AnyUserData = lua.load(code).eval().unwrap();
        ud.borrow::<T>().unwrap().clone()
    }

    #[test]
    fn builders_chain() {
        let lua = lua();
        let p: WParagraph = get(
            &lua,
            r#"return ui.Paragraph("a\nb"):wrap("trim"):scroll(2):fg("cyan")
                   :block(ui.Border(ui.Edge.TOP):type(ui.Border.ROUNDED):title("T"))"#,
        );
        assert_eq!((p.wrap, p.scroll), (WWrap::Trim, 2));
        assert_eq!(p.style.fg.as_deref(), Some("cyan"));
        let block = p.block.unwrap();
        assert_eq!((block.edges, block.kind), (edge::TOP, WBorderType::Rounded));
        let g: WGauge = get(&lua, r#"return ui.Gauge():percent(250):label("x")"#);
        assert_eq!(g.ratio, 1.0);
        assert!(
            lua.load(r#"ui.Paragraph("x"):wrap("sideways")"#)
                .exec()
                .is_err()
        );
    }

    #[test]
    fn tables_and_lists_in_both_forms() {
        let lua = lua();
        let old: WTable = get(
            &lua,
            r#"return ui.Table({"h1", "h2"}, {{"a", "b"}, {"c", "d"}})"#,
        );
        assert_eq!(old.header.unwrap().cells.len(), 2);
        assert_eq!(old.rows.len(), 2);
        let new: WTable = get(
            &lua,
            r#"return ui.Table({ ui.Row({"x", ui.Line("y")}):height(2) })
                   :header({"H"}):widths({ ui.Constraint.Length(3), ui.Constraint.Fill(1) })"#,
        );
        assert_eq!((new.rows[0].height, new.widths.len()), (2, 2));
        let list: WList = get(&lua, r#"return ui.List({"a", ui.Span("b")})"#);
        assert_eq!(list.items.len(), 2);
        assert!(
            matches!(list.items[1].body, WTextBody::Lines(ref l) if l[0].spans[0].content == "b")
        );
    }
}
