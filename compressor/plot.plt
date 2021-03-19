set term wxt size 1440,900 persist
set grid
plot "log.txt" u 1:2 w l, "log2.txt" u 1:2 w l
