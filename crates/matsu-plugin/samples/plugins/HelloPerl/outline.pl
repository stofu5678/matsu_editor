# The jack Outline: a tree of the lines that start with "#".
Outline->SetListType(100);
Outline->SetTitle('Headings (Perl sample)');
my $total = Editor->GetLineCount(0);
for my $n (1 .. $total) {
    my $text = Editor->GetLineStr($n);
    $text =~ s/[\r\n]+\z//;
    Outline->AddFuncInfo2($n, 1, $2, length($1) - 1) if $text =~ /^(#+)\s*(.*)$/;
}
