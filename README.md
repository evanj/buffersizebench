# Buffer Size Benchmark

This is attempting to determine how much the buffer size passed to the `read()` system call matters for different data sources on a "bulk transfer" workload.

On a GCP T2D instance with Kernel 6.2.0-1005-gcp from Ubuntu 23.04

AMD EPYC 7B13 MHz: 2449.998

## Unix sockets

For really large buffers, reads will often be "short" due to the kernel buffers. Arguably this is "realistic", since we are using a real writer and reader. This basically means there are diminishing returns to larger application buffers, which is what we are looking for anyway.

### Mac OS X

The default unix buffer is 8192 bytes, and results in really slow default performance. Localhost TCP is much faster by default. Calling `setsockopt(sock, SOL_SOCKET, SO_SNDBUF, ...)` to increase the buffer size makes local Unix sockets work much more efficiently. TODO: include some results.


## /dev/zero

Best = 128KiB; 64KiB is very close
These numbers are a bit variable. On one run the best was 256 KiB, but only slightly.

```
buf_size=1; duration=1.936615637s; num_syscalls=12033694; 5.9 MiB/s; 6213775.1 syscalls/s
buf_size=2; duration=2.019729647s; num_syscalls=12578616; 11.9 MiB/s; 6227871.2 syscalls/s
buf_size=4; duration=2.003707709s; num_syscalls=12618296; 24.0 MiB/s; 6297473.4 syscalls/s
buf_size=8; duration=2.016048434s; num_syscalls=12594458; 47.7 MiB/s; 6247100.9 syscalls/s
buf_size=16; duration=1.988772481s; num_syscalls=12247397; 94.0 MiB/s; 6158269.5 syscalls/s
buf_size=32; duration=2.005336743s; num_syscalls=11947431; 181.8 MiB/s; 5957817.8 syscalls/s
buf_size=64; duration=1.993962331s; num_syscalls=11242270; 344.1 MiB/s; 5638155.7 syscalls/s
buf_size=128; duration=1.994401742s; num_syscalls=10672358; 653.2 MiB/s; 5351157.6 syscalls/s
buf_size=256; duration=1.995320541s; num_syscalls=10626992; 1300.3 MiB/s; 5325957.3 syscalls/s
buf_size=512; duration=2.010810532s; num_syscalls=10487676; 2546.7 MiB/s; 5215646.0 syscalls/s
buf_size=1024; duration=1.98798139s; num_syscalls=10172939; 4997.3 MiB/s; 5117220.4 syscalls/s
buf_size=2048; duration=1.974627069s; num_syscalls=9624639; 9519.8 MiB/s; 4874155.3 syscalls/s
buf_size=4096; duration=1.974626349s; num_syscalls=8768084; 17345.2 MiB/s; 4440376.3 syscalls/s
buf_size=8192; duration=1.97347823s; num_syscalls=6234413; 24680.5 MiB/s; 3159098.9 syscalls/s
buf_size=16384; duration=1.967566059s; num_syscalls=3927729; 31191.2 MiB/s; 1996237.4 syscalls/s
buf_size=32768; duration=1.973112478s; num_syscalls=2208480; 34977.7 MiB/s; 1119287.4 syscalls/s
buf_size=65536; duration=1.97939128s; num_syscalls=1201923; 37951.2 MiB/s; 607218.5 syscalls/s
buf_size=131072; duration=1.976409639s; num_syscalls=622936; 39398.2 MiB/s; 315185.7 syscalls/s
buf_size=262144; duration=1.928532866s; num_syscalls=303572; 39352.7 MiB/s; 157410.9 syscalls/s
buf_size=524288; duration=1.959564379s; num_syscalls=147348; 37597.1 MiB/s; 75194.3 syscalls/s
buf_size=1048576; duration=1.858691312s; num_syscalls=63920; 34389.8 MiB/s; 34389.8 syscalls/s
buf_size=2097152; duration=1.991255951s; num_syscalls=34749; 34901.6 MiB/s; 17450.8 syscalls/s
buf_size=4194304; duration=1.936849826s; num_syscalls=15998; 33039.2 MiB/s; 8259.8 syscalls/s
buf_size=8388608; duration=2.027502033s; num_syscalls=8515; 33598.0 MiB/s; 4199.7 syscalls/s
buf_size=16777216; duration=1.928788426s; num_syscalls=3499; 29025.5 MiB/s; 1814.1 syscalls/s
```


## /dev/urandom

Best = 16KiB; 8KiB is very close

```
buf_size=1; duration=1.847687228s; num_syscalls=5102040; 2.6 MiB/s; 2761311.5 syscalls/s
buf_size=2; duration=2.010906443s; num_syscalls=5496015; 5.2 MiB/s; 2733103.3 syscalls/s
buf_size=4; duration=1.98661035s; num_syscalls=5482456; 10.5 MiB/s; 2759703.7 syscalls/s
buf_size=8; duration=1.94713998s; num_syscalls=5370569; 21.0 MiB/s; 2758183.3 syscalls/s
buf_size=16; duration=1.990003339s; num_syscalls=5460154; 41.9 MiB/s; 2743791.4 syscalls/s
buf_size=32; duration=2.057340598s; num_syscalls=5494505; 81.5 MiB/s; 2670683.2 syscalls/s
buf_size=64; duration=2.003897077s; num_syscalls=4127881; 125.7 MiB/s; 2059926.7 syscalls/s
buf_size=128; duration=2.043772338s; num_syscalls=3175611; 189.7 MiB/s; 1553798.8 syscalls/s
buf_size=256; duration=1.673637634s; num_syscalls=1824151; 266.1 MiB/s; 1089931.9 syscalls/s
buf_size=512; duration=1.782337682s; num_syscalls=1187789; 325.4 MiB/s; 666422.0 syscalls/s
buf_size=1024; duration=1.863302763s; num_syscalls=701705; 367.8 MiB/s; 376592.0 syscalls/s
buf_size=2048; duration=1.873606314s; num_syscalls=377921; 394.0 MiB/s; 201707.8 syscalls/s
buf_size=4096; duration=1.941973154s; num_syscalls=202326; 407.0 MiB/s; 104185.8 syscalls/s
buf_size=8192; duration=1.966763905s; num_syscalls=104382; 414.6 MiB/s; 53073.0 syscalls/s
buf_size=16384; duration=1.982616664s; num_syscalls=52986; 417.6 MiB/s; 26725.3 syscalls/s
buf_size=32768; duration=2.000773776s; num_syscalls=26563; 414.9 MiB/s; 13276.4 syscalls/s
buf_size=65536; duration=1.993078445s; num_syscalls=13213; 414.3 MiB/s; 6629.4 syscalls/s
buf_size=131072; duration=2.002164246s; num_syscalls=6642; 414.7 MiB/s; 3317.4 syscalls/s
buf_size=262144; duration=2.005515216s; num_syscalls=3321; 414.0 MiB/s; 1655.9 syscalls/s
buf_size=524288; duration=1.990772375s; num_syscalls=1647; 413.7 MiB/s; 827.3 syscalls/s
buf_size=1048576; duration=1.998057765s; num_syscalls=827; 413.9 MiB/s; 413.9 syscalls/s
buf_size=2097152; duration=2.001743165s; num_syscalls=415; 414.6 MiB/s; 207.3 syscalls/s
buf_size=4194304; duration=1.997274741s; num_syscalls=205; 410.6 MiB/s; 102.6 syscalls/s
buf_size=8388608; duration=1.98669871s; num_syscalls=101; 406.7 MiB/s; 50.8 syscalls/s
buf_size=16777216; duration=3.956543559s; num_syscalls=100; 404.4 MiB/s; 25.3 syscalls/s
```
