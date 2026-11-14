# The jack SmartIndent: after Enter, four more spaces when the line above ends with a colon.
my $typed = Indent->GetChar;
if ($typed eq "\r" || $typed eq "\n") {
    my $line = int(Editor->ExpandParameter('$y'));
    if ($line >= 2) {
        my $above = Editor->GetLineStr($line - 1);
        $above =~ s/[\r\n]+\z//;
        Editor->InsText('    ') if $above =~ /:\z/;
    }
}
