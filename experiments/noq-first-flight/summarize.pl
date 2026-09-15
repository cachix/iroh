use strict;
use warnings;
use POSIX qw(ceil);
use FindBin;
my $results_dir=$ARGV[0] // "$FindBin::Bin/reproduced";
my (%samples,%bulk,%runs);
my ($total,$files)=(0,0);
for my $file (glob("$results_dir/bulk*-repeat*-*.csv")) {
    $file =~ m{/bulk([01])-repeat([12])-(control|burst2|burst4|burst8)\.csv$} or next;
    my ($compete,$repeat,$variant)=($1,$2,$3);
    open my $in,'<',$file or die $!;
    my $count=0;
    while (<$in>) {
        next if /^suite/;
        chomp;
        my ($suite,$sample,$us,$ctx,$stx,$status)=split /,/;
        die "failed sample: $file" unless $status eq 'ok';
        push @{$samples{"$compete|$variant|$suite"}},$us/1000;
        push @{$runs{"$compete|$variant|$repeat|$suite"}},$us/1000;
        ++$count;
    }
    $total+=$count; ++$files;
    die "incomplete count: $file $count" if $ENV{REQUIRE_COMPLETE} && $count!=40;
    (my $log=$file)=~s/\.csv$/.log/;
    open my $logs,'<',$log or die $!;
    my $complete=0;
    while (<$logs>) {
        die "run failure $log" if /panicked|handshake failed/;
        if (/BULK_RESULT seconds=([\d.]+) bytes=(\d+) mbps=([\d.]+) handshakes_per_second=([\d.]+)/) {
            push @{$bulk{"$compete|$variant"}},[$1,$2,$3,$4];
            $complete=1;
        }
    }
    die "no completion: $file" if $ENV{REQUIRE_COMPLETE} && !$complete;
}
die "wrong totals $total / $files" if $ENV{REQUIRE_COMPLETE} && ($total!=480 || $files!=12);
print "| Competition | Variant | Identity | N | Median ms | p95 ms | >300 ms |\n|---|---|---|---:|---:|---:|---:|\n";
for my $key (sort keys %samples) {
    my @v=sort {$a<=>$b} @{$samples{$key}};
    my $n=@v;
    printf "| %s | %d | %.2f | %.2f | %d |\n",join(' | ',split /\|/,$key),$n,($v[int(($n-1)/2)]+$v[int($n/2)])/2,$v[ceil(.95*$n)-1],scalar(grep {$_>300} @v);
}
print "\n| Competition | Variant | Runs | Bulk Mbit/s | Range | Attempts/s range |\n|---|---|---:|---:|---|---|\n";
for my $key (sort keys %bulk) {
    my ($t,$total_bytes)=(0,0); my (@rates,@loads);
    for (@{$bulk{$key}}) { $t+=$_->[0]; $total_bytes+=$_->[1]; push @rates,$_->[2]; push @loads,$_->[3]; }
    @rates=sort {$a<=>$b} @rates; @loads=sort {$a<=>$b} @loads;
    printf "| %s | %d | %.4f | %.4f–%.4f | %.4f–%.4f |\n",join(' | ',split /\|/,$key),scalar(@rates),$total_bytes*8/$t/1e6,$rates[0],$rates[-1],$loads[0],$loads[-1];
}
print "\nCompleted sample rows: $total across $files files.\n";
