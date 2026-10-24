#
# The boot library of the Perl macros. The editor starts `perl -` and writes this text (after
# api_gen.pl) to the standard input, ending with `__END__`; Perl reads the program up to there,
# and the rest of the standard input is the DATA handle: the `initialize` line first, then the
# answers to the calls. So there is no file to open, and a Windows perl, which opens a file with
# the ANSI code page, has no path to fail on.
#
# It reads and writes the editor's JSON-RPC lines synchronously (docs/poc/json-rpc.md, spec
# sections 2 and 3). The first line holds the user's script (params.script, as characters);
# it is run with `eval`, with a `#line` directive, so the errors show the script's own name and
# line. The class `Editor` (api_gen.pl) and `EditorError` are ready, so the script needs no
# `use`. The JSON is JSON::PP, which is in the core of Perl since 5.14.
package EditorError;
use strict;
use warnings;
use overload '""' => \&as_string, fallback => 1;

# The editor answered a call with an error (a JSON-RPC error object). `code` is its code.
sub new {
    my ($class, $code, $message, $file, $line) = @_;
    return bless { code => $code, message => $message, file => $file, line => $line }, $class;
}
sub code    { return $_[0]{code} }
sub message { return $_[0]{message} }
sub file    { return $_[0]{file} }
sub line    { return $_[0]{line} }

# Like a die message of Perl: "... at FILE line N.\n"
sub as_string {
    my ($self) = @_;
    return "EditorError $self->{code}: $self->{message} at $self->{file} line $self->{line}.\n";
}

package MatsuBoot;
use strict;
use warnings;
use B ();
use JSON::PP ();
use Scalar::Util ();

# Bytes in and out, so that Windows does not turn the line break into CR LF and the user's
# output is UTF-8 whatever the console code page is. A string with a character above U+00FF is
# written as UTF-8; the script's strings are characters.
binmode(STDOUT, ':raw:utf8');
binmode(STDERR, ':raw:utf8');
binmode(main::DATA, ':raw');
STDOUT->autoflush(1);
STDERR->autoflush(1);

my $RS = "\x1e";
my $json = JSON::PP->new->ascii->canonical->allow_nonref;
my $next_id = 1;

# Perl keeps the name of a file as bytes, and the name in a `#line` directive is no exception: a
# name with Japanese characters comes back from `caller` and from the "at FILE line N." of a
# message as UTF-8 bytes, which the `:utf8` layer of STDERR would encode again. `fix_name` turns
# those bytes back into the characters of the name. Set below, for a name that is not ASCII.
my ($name_as_bytes, $name_as_chars);

sub fix_name {
    my ($text) = @_;
    return $text if !defined $text || ref $text || !defined $name_as_bytes;
    $text =~ s/\Q$name_as_bytes\E/$name_as_chars/g;
    return $text;
}

sub read_line {
    # The script may have changed the record separator (`local $/;` to read a file whole).
    local $/ = "\n";
    my $line = <main::DATA>;
    return $line;
}

# An argument as the JSON value it stands for. Perl does not tell a number from a string for
# sure: a value is a number when it holds a number and, if it also holds a string (it was
# printed, for example), the string reads as a number. A string that was never used as a number
# stays a string, and "$x" makes a string of anything.
sub to_json_value {
    my ($value, $file, $line) = @_;
    return undef unless defined $value;
    if (ref $value) {
        return $value if Scalar::Util::blessed($value) && $value->isa('JSON::PP::Boolean');
        die EditorError->new(-32602, 'cannot send a ' . ref($value) . ' reference to the editor', $file, $line);
    }
    my $flags = B::svref_2object(\$value)->FLAGS;
    if (($flags & (B::SVp_IOK | B::SVp_NOK))
        && (!($flags & B::SVp_POK) || Scalar::Util::looks_like_number($value))) {
        # NaN and infinity are not JSON.
        if ($value != $value || $value == 9**9**9 || $value == -9**9**9) {
            die EditorError->new(-32602, 'cannot send NaN or infinity to the editor', $file, $line);
        }
        return 0 + $value;
    }
    return "$value";
}

# Sends a request and waits for its response. Called from a method of `Editor`, which is called
# from the user's code: the frame above this one has the user's file and line.
sub call {
    my ($method, @args) = @_;
    my (undef, $file, $line) = caller(1);
    $file = defined $file ? fix_name($file) : '?';
    $line = 0 unless defined $line;
    my @params = map { to_json_value($_, $file, $line) } @args;
    my $id = $next_id++;
    my $text = $json->encode({ jsonrpc => '2.0', id => $id, method => $method, params => \@params });
    # And the output separators of `print` (`$,` and `$\`).
    local ($,, $\) = (undef, undef);
    print STDOUT "$RS$text\n";
    while (1) {
        my $reply = read_line();
        die EditorError->new(-32000, "The editor closed the connection before it answered $method", $file, $line)
            unless defined $reply;
        $reply =~ s/^\s+|\s+$//g;
        next if $reply eq '';
        my $message = eval { $json->decode($reply) };
        # A notification, or an answer to something that is no longer waited for.
        next unless ref $message eq 'HASH' && defined $message->{id} && $message->{id} == $id;
        if (ref $message->{error} eq 'HASH') {
            my $error = $message->{error};
            die EditorError->new($error->{code}, $error->{message}, $file, $line);
        }
        return $message->{result};
    }
}

# The first line is the initialize notification.
my $info;
my $first = read_line();
if (defined $first && $first =~ /\S/) {
    my $message = eval { $json->decode($first) };
    $info = $message->{params} if ref $message eq 'HASH' && ($message->{method} // '') eq 'initialize';
}
my $script = ref $info eq 'HASH' ? $info->{script} : undef;
unless (ref $script eq 'HASH') {
    print STDERR "the editor did not send the script in initialize\n";
    exit 1;
}

$Editor::CALL = \&call;
$Editor::INFO = $info;

# The name as `#line` gets it (MatsuUser::run turns quotes and line breaks into apostrophes).
{
    my $name = $script->{name};
    $name =~ s/["\n\r]/'/g;
    if ($name =~ /[^\x00-\x7f]/) {
        $name_as_chars = $name;
        $name_as_bytes = $name;
        utf8::encode($name_as_bytes);
        # As characters, each byte one: what the message holds after it joined the bytes in.
        utf8::upgrade($name_as_bytes);
        # The messages that Perl and the script make ("... at FILE line N.") say the name right.
        $SIG{__WARN__} = sub { print STDERR map { fix_name($_) } @_ };
        $SIG{__DIE__} = sub { die(@_) if ref $_[0]; die fix_name($_[0]) };
    }
}

# The text of the script is characters (the JSON was decoded), so the literals in it are too.
my $ok = MatsuUser::run($script->{source}, $script->{name});
my $error = $@;
unless ($ok) {
    if (ref $error || (defined $error && $error ne '')) {
        $error = fix_name("$error");
        $error .= "\n" unless $error =~ /\n\z/;
        print STDERR $error;
        exit 1;
    }
}
exit 0;
