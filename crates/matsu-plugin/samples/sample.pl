# Sample macro (Perl: .pl, Perl 5.14 or later).
# `Editor` is there without a `use`. The names and the arguments are Sakura's macro API, as class
# methods: Editor->InsText('x'). The script is run as characters (as if under `use utf8`), and
# the output is UTF-8.
use strict;
use warnings;

Editor->GoFileTop();
Editor->InsText("# サンプルマクロ 😀\n");

my $count = Editor->GetLineCount(0);
my $file = Editor->GetFilename() || '(無題)';
print "行数: $count ファイル: $file\n";
for my $i (1 .. $count) {
    my $line = Editor->GetLineStr($i);
    $line =~ s/\n\z//;
    print "$i: $line\n";
}

# Select line 3 (a line number and a column both start at 1) and replace it.
Editor->MoveCursor(3, 1, 0);
Editor->MoveCursor(3, 1000, 1);    # past the end of the line: the end of the line
print '選択: ', Editor->GetSelectedString(0), ' / IsTextSelected = ', Editor->IsTextSelected(), "\n";
Editor->InsText('みかん😀 (置き換えました)');

# An error response is an exception: an EditorError object with a code.
eval { Editor->GetLineStr('not a number'); 1 } or do {
    my $e = $@;
    print 'エラー: ', $e->code, ' ', $e->message, "\n" if ref $e && $e->isa('EditorError');
};

Editor->AddTail("-- マクロの終わり --\n");
Editor->Jump(1);
