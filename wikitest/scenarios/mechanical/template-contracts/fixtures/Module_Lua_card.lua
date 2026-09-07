local p = {}
function p.main(frame)
    local args = frame:getParent().args
    return table.concat({args.label or args.text or '', args.count or '', args.color or ''}, ' ')
end
return p
