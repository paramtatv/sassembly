#!/usr/bin/perl
# tools/with-timeout.pl SECS CMD [ARGS...] — run CMD in its OWN process group; on timeout kill ONLY that group (TERM, then KILL),
# print "TIMEOUT after Ns: CMD" on stderr and exit 124. Otherwise exit with CMD's status (128+signal if it died of one).
# Portable (perl ships with macOS and Linux). Never matches processes by name: it signals the pgid it created.
use strict; use warnings;
my $secs = shift @ARGV; die "usage: with-timeout.pl SECS CMD [ARGS...]\n" unless defined $secs && $secs =~ /^\d+$/ && @ARGV;
my $pid = fork(); die "fork: $!\n" unless defined $pid;
if ($pid == 0) { setpgrp(0, 0); exec { $ARGV[0] } @ARGV or do { print STDERR "with-timeout: cannot exec $ARGV[0]: $!\n"; exit 127 } }
$SIG{ALRM} = sub {
    kill 'TERM', -$pid; select(undef, undef, undef, 2); kill 'KILL', -$pid; waitpid($pid, 0);
    print STDERR "TIMEOUT after ${secs}s: @ARGV\n"; exit 124;
};
alarm $secs; waitpid($pid, 0); alarm 0;
my $st = $?; exit(($st & 127) ? 128 + ($st & 127) : ($st >> 8));
