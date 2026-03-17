discover
mount 0
load_proj afifo_test
program 0
attach 0
tick_file write_bin.txt
# Optional: run a few normal idle ticks afterwards
tick 4 0x0400
detach
