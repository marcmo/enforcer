def expand_tabs(s, tab_stops = 8)
  puts s
  s.gsub(/([^\t\n]*)\t/) do
    $1 + "-" * (tab_stops - ($1.size % tab_stops))
  end
end

puts expand_tabs(" \t", 2)
puts expand_tabs("\t", 2)
puts expand_tabs(" \t  \t", 2)
puts expand_tabs("abc \tdef", 2)
puts expand_tabs("abc \tdef", 2)
puts expand_tabs("\t B", 2)
puts expand_tabs("\t  B", 2)
puts expand_tabs("\t   B", 2)
puts expand_tabs("\t    B", 2)
puts expand_tabs("\t     B", 2)
puts expand_tabs("\t      B", 2)
puts expand_tabs("      \tB", 2)
puts expand_tabs("     \t B", 2)
puts expand_tabs("    \t  B", 2)
puts expand_tabs("   \t   B", 2)
puts expand_tabs("  \t    B", 2)
puts expand_tabs(" \t     B", 2)
puts expand_tabs("\t      B", 2)
puts expand_tabs("foo\tbar\tbaz", 4)

