use strict;
use warnings;
$|=1;
print "epoch,cpu_busy_percent,cpu_pressure_avg10,load1\n";
my ($last_total,$last_idle);
while (1) {
    open my $stat,'<','/proc/stat' or die $!;
    my @cpu=split /\s+/,scalar(<$stat>);
    my $total=0; $total+=$cpu[$_] for 1..8;
    my $idle=$cpu[4]+$cpu[5];
    open my $pressure,'<','/proc/pressure/cpu' or die $!;
    my $line=<$pressure>; $line =~ /avg10=([\d.]+)/ or die 'pressure format';
    my $psi=$1;
    open my $load,'<','/proc/loadavg' or die $!;
    my ($load1)=split /\s+/,scalar(<$load>);
    if (defined $last_total && $total>$last_total) {
        printf "%d,%.3f,%s,%s\n",time,100*(1-($idle-$last_idle)/($total-$last_total)),$psi,$load1;
    }
    ($last_total,$last_idle)=($total,$idle);
    sleep 5;
}
