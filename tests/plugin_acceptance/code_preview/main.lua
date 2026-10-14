--- @since 1.3.0
-- Acceptance port of a code previewer: a header line with the file name,
-- size and permissions (File inherits Cha), a bar under it, and the
-- highlighted code of `pairee.preview_code` below, laid out with
-- `Layout:split` and placed with `:area`.
local M = {}

function M:peek(job)
    local ui = pairee.ui
    local parts = ui.Layout()
        :direction(ui.Layout.VERTICAL)
        :constraints({ ui.Constraint.Length(1), ui.Constraint.Length(1), ui.Constraint.Fill(1) })
        :split(job.area)
    local header, body = parts[1], parts[3]
    local rule = ui.Rect({ x = job.area.x, y = header.bottom, w = job.area.w, h = 1 })
    local code, err = pairee.preview_code({ file = job.file, skip = job.skip, lines = body.h })
    if code == nil then
        return ui.Paragraph(tostring(err))
    end
    local title = ui.Line({
        ui.Span(job.file.name):bold(),
        " ",
        ui.Span(tostring(job.file.len) .. " B"),
        " ",
        ui.Span(job.file:perm() or ""),
    })
    return {
        ui.Clear(job.area),
        ui.Fill(" "):area(header),
        ui.Paragraph(title):area(header),
        ui.Bar(ui.Edge.TOP):symbol("-"):area(rule),
        code:area(body:pad(ui.Pad.left(1))),
    }
end

return M
