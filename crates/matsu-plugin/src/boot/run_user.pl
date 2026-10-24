# Runs the user's script of a Perl macro. This comes first in the program, before any `my` or
# `our`, so the `eval` sees none of the boot's variables (a lexical is visible only below its
# declaration), and it has no `strict` or `warnings`: a script without them runs as it does under
# `perl script.pl`. The script's name goes into a `#line` directive, so the errors show the
# script's own name and line.
package MatsuUser;

sub run {
    my $name = $_[1];
    $name =~ s/["\n\r]/'/g;
    return eval "package main;\n#line 1 \"$name\"\n$_[0]\n;1";
}

