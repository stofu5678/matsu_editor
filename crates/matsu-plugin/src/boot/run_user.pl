# Runs the user's script of a Perl macro. This comes first in the program, before any `my` or
# `our`, so the `eval` sees none of the boot's variables (a lexical is visible only below its
# declaration), and it has no `strict` or `warnings`: a script without them runs as it does under
# `perl script.pl`. The script's name goes into a `#line` directive, so the errors show the
# script's own name and line.
package MatsuUser;

# Takes the text and the name for `#line` (the boot has made a name that the directive can hold).
# No `my` here: a variable of this sub would be visible to the script.
sub run {
    return eval "package main;\n#line 1 \"$_[1]\"\n$_[0]\n;1";
}

