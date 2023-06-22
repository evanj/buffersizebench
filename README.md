# Buffer Size Benchmark

This is attempting to determine how much the buffer size passed to the `read()` system call matters for different data sources on a "bulk transfer" workload. I did very little tuning, so this should represent "out of the box" performance, but the results will vary substantially from kernel to kernel and machine to machine.

*Rough Conclusion*: Read buffers: 16 KiB is a reasonable default. 8 KiB might even be enough. Write buffers: Bigger seems better. Waiting on more data.

Results are from a 16 core GCP T2D instance with Kernel 6.2.0-1005-gcp from Ubuntu 23.04
/proc/cpuinfo reports "AMD EPYC 7B13 MHz: 2449.998"

## Results summary

* Larger buffers do decrease system call overhead, and generally improve throughput. However, past a certain point, throughput seems to decrease. Maybe the CPU cache gets tool large.
* The "best" buffer size depends on the system call, but a buffer of about 16 kiB is probably enough.
* Mac OS X: The default Unix socket buffer is 8192 and results in low throughput. Localhost TCP sockets are ~3X faster by default. Calling `setsockopt(sock, SOL_SOCKET, SO_SNDBUF, ...)` to increase the buffer size makes local Unix sockets work much more efficiently. TODO: include some results.
* Manually tuning the SO_SNDBUF and SO_RCVBUF settings for localhost TCP sockets doesn't seem to help.
* The results vary a lot for sender/receiver pairs at high throughput. I suspect the kernel scheduling has a very large impact. You can see this in the "short reads" counts. If the kernel happens to schedule things so the read() call returns full payloads, throughput is good. If it returns smaller payloads, throughput is less good.


## GCP Network Bandwidth

"The maximum egress bandwidth is generally 2 Gbps per vCPU" from https://cloud.google.com/compute/docs/network-bandwidth#vm-out

Need 16 cores to hit the 32 Gbps limit


## Unix sockets

For really large buffers, reads will often be "short" due to the kernel buffers. Arguably this is "realistic", since we are using a real writer and reader. This basically means there are diminishing returns to larger application buffers, which is what we are looking for anyway.

Best = 16 kiB = 8817.2 MiB/s

```
buf_size=1; duration=1.508913262s; num_syscalls=6583278; 4.2 MiB/s; 4362926.7 syscalls/s; short_reads=0
buf_size=2; duration=1.938499729s; num_syscalls=8382230; 8.2 MiB/s; 4324081.1 syscalls/s; short_reads=0
buf_size=4; duration=2.109093394s; num_syscalls=9289364; 16.8 MiB/s; 4404434.6 syscalls/s; short_reads=0
buf_size=8; duration=2.106743724s; num_syscalls=9263548; 33.5 MiB/s; 4397093.0 syscalls/s; short_reads=0
buf_size=16; duration=1.893588446s; num_syscalls=8045053; 64.8 MiB/s; 4248575.2 syscalls/s; short_reads=0
buf_size=32; duration=2.113735335s; num_syscalls=9013069; 130.1 MiB/s; 4264048.0 syscalls/s; short_reads=0
buf_size=64; duration=1.99757676s; num_syscalls=8385393; 256.2 MiB/s; 4197782.6 syscalls/s; short_reads=0
buf_size=128; duration=2.211035916s; num_syscalls=8737440; 482.4 MiB/s; 3951740.4 syscalls/s; short_reads=0
buf_size=256; duration=2.205613006s; num_syscalls=8254234; 913.7 MiB/s; 3742376.4 syscalls/s; short_reads=9
buf_size=512; duration=1.70226894s; num_syscalls=6136862; 1760.3 MiB/s; 3605107.2 syscalls/s; short_reads=18
buf_size=1024; duration=1.941036403s; num_syscalls=6504097; 3272.3 MiB/s; 3350837.2 syscalls/s; short_reads=55
buf_size=2048; duration=1.906473243s; num_syscalls=5317469; 5442.1 MiB/s; 2789165.3 syscalls/s; short_reads=11736
buf_size=4096; duration=1.720335621s; num_syscalls=3286452; 7444.7 MiB/s; 1910355.1 syscalls/s; short_reads=21315
buf_size=8192; duration=2.055856904s; num_syscalls=2064421; 7735.6 MiB/s; 1004165.7 syscalls/s; short_reads=76180
buf_size=16384; duration=1.63917513s; num_syscalls=980111; 8817.2 MiB/s; 597929.4 syscalls/s; short_reads=115610
buf_size=32768; duration=1.819850512s; num_syscalls=581552; 8510.6 MiB/s; 319560.3 syscalls/s; short_reads=137099
buf_size=65536; duration=2.046739364s; num_syscalls=331912; 8118.0 MiB/s; 162166.2 syscalls/s; short_reads=138801
buf_size=131072; duration=2.979510345s; num_syscalls=244071; 6626.0 MiB/s; 81916.5 syscalls/s; short_reads=191097
buf_size=262144; duration=2.48543555s; num_syscalls=192428; 7471.0 MiB/s; 77422.2 syscalls/s; short_reads=192133
buf_size=524288; duration=2.440867278s; num_syscalls=193360; 7644.9 MiB/s; 79217.7 syscalls/s; short_reads=193344
buf_size=1048576; duration=2.400300958s; num_syscalls=210026; 8402.3 MiB/s; 87499.9 syscalls/s; short_reads=210020
buf_size=2097152; duration=1.71888937s; num_syscalls=138784; 7679.9 MiB/s; 80740.5 syscalls/s; short_reads=138782
buf_size=4194304; duration=2.606114162s; num_syscalls=217406; 7714.3 MiB/s; 83421.5 syscalls/s; short_reads=217406
buf_size=8388608; duration=2.195294766s; num_syscalls=203577; 8990.7 MiB/s; 92733.3 syscalls/s; short_reads=203577
buf_size=16777216; duration=2.113753235s; num_syscalls=203752; 7825.3 MiB/s; 96393.5 syscalls/s; short_reads=203752
```

## TCP Localhost

Best = 4 kiB = 6511.7 MiB/s

```
buf_size=1; duration=1.512511567s; num_syscalls=5274261; 3.3 MiB/s; 3487088.0 syscalls/s; short_reads=0
buf_size=2; duration=2.073014755s; num_syscalls=7225434; 6.6 MiB/s; 3485471.6 syscalls/s; short_reads=1
buf_size=4; duration=2.102940015s; num_syscalls=7238509; 13.1 MiB/s; 3442090.1 syscalls/s; short_reads=0
buf_size=8; duration=1.788082031s; num_syscalls=6142507; 26.2 MiB/s; 3435249.0 syscalls/s; short_reads=0
buf_size=16; duration=1.998296984s; num_syscalls=6738545; 51.5 MiB/s; 3372143.9 syscalls/s; short_reads=0
buf_size=32; duration=2.062306765s; num_syscalls=7119972; 105.4 MiB/s; 3452431.1 syscalls/s; short_reads=0
buf_size=64; duration=1.910036812s; num_syscalls=6464125; 206.6 MiB/s; 3384293.4 syscalls/s; short_reads=0
buf_size=128; duration=1.909269703s; num_syscalls=6242198; 399.1 MiB/s; 3269416.6 syscalls/s; short_reads=0
buf_size=256; duration=1.944395473s; num_syscalls=6250000; 784.8 MiB/s; 3214366.7 syscalls/s; short_reads=0
buf_size=512; duration=2.239501786s; num_syscalls=6860829; 1495.9 MiB/s; 3063551.5 syscalls/s; short_reads=0
buf_size=1024; duration=2.128489395s; num_syscalls=5752098; 2639.1 MiB/s; 2702432.1 syscalls/s; short_reads=163
buf_size=2048; duration=6.164423795s; num_syscalls=5868394; 1857.7 MiB/s; 951977.7 syscalls/s; short_reads=183407
buf_size=4096; duration=742.017215ms; num_syscalls=1236966; 6511.7 MiB/s; 1667031.4 syscalls/s; short_reads=408
buf_size=8192; duration=4.527859035s; num_syscalls=2833010; 4885.8 MiB/s; 625684.2 syscalls/s; short_reads=16213
buf_size=16384; duration=9.993667934s; num_syscalls=1536455; 2400.8 MiB/s; 153742.9 syscalls/s; short_reads=206051
buf_size=32768; duration=1.964837535s; num_syscalls=121000; 1920.7 MiB/s; 61582.7 syscalls/s; short_reads=60421
buf_size=65536; duration=1.897023326s; num_syscalls=60067; 1971.3 MiB/s; 31663.8 syscalls/s; short_reads=59951
buf_size=131072; duration=1.915982115s; num_syscalls=60648; 1972.8 MiB/s; 31653.7 syscalls/s; short_reads=60640
buf_size=262144; duration=1.905815296s; num_syscalls=60506; 1977.3 MiB/s; 31748.1 syscalls/s; short_reads=60505
buf_size=524288; duration=1.981485934s; num_syscalls=63818; 2007.5 MiB/s; 32207.1 syscalls/s; short_reads=63817
buf_size=1048576; duration=2.035394905s; num_syscalls=64616; 1978.3 MiB/s; 31746.2 syscalls/s; short_reads=64615
buf_size=2097152; duration=2.170965843s; num_syscalls=61080; 1752.5 MiB/s; 28134.9 syscalls/s; short_reads=61080
buf_size=4194304; duration=1.984369545s; num_syscalls=51118; 1604.6 MiB/s; 25760.3 syscalls/s; short_reads=51118
buf_size=8388608; duration=1.979897295s; num_syscalls=50809; 1598.0 MiB/s; 25662.4 syscalls/s; short_reads=50809
buf_size=16777216; duration=1.872009035s; num_syscalls=51228; 1705.5 MiB/s; 27365.3 syscalls/s; short_reads=51228
```

## TCP Remote

Best = 1 KiB = 1449.7 MiB/s

16 core GCP VM documented as having a 32 Gbps limit = 3814 MiB/s. Using `iperf` reaches that limit with 3 or 4 TCP flows. With 1 TCP flow it hits 1425 MiB/s, which is basically the same as this test.

```
TCP writer_addr=10.128.0.39:12345; reader_sock SO_RCVBUF=131072:
TCP reader sock; SO_RCVBUF=131072
buf_size=1; duration=1.47981648s; num_syscalls=4925987; 3.2 MiB/s; 3328782.4 syscalls/s; short_reads=0
buf_size=2; duration=2.076351134s; num_syscalls=6858711; 6.3 MiB/s; 3303252.0 syscalls/s; short_reads=1
buf_size=4; duration=2.233437932s; num_syscalls=7307271; 12.5 MiB/s; 3271759.2 syscalls/s; short_reads=0
buf_size=8; duration=2.087131985s; num_syscalls=6624711; 24.2 MiB/s; 3174073.8 syscalls/s; short_reads=0
buf_size=16; duration=1.957479169s; num_syscalls=6016848; 46.9 MiB/s; 3073773.7 syscalls/s; short_reads=0
buf_size=32; duration=2.065826752s; num_syscalls=6682259; 98.7 MiB/s; 3234665.7 syscalls/s; short_reads=0
buf_size=64; duration=1.994596384s; num_syscalls=6416427; 196.3 MiB/s; 3216905.0 syscalls/s; short_reads=0
buf_size=128; duration=2.322409574s; num_syscalls=7079647; 372.1 MiB/s; 3048405.9 syscalls/s; short_reads=0
buf_size=256; duration=1.834770973s; num_syscalls=5393745; 717.7 MiB/s; 2939737.5 syscalls/s; short_reads=1
buf_size=512; duration=2.324527454s; num_syscalls=6238505; 1310.4 MiB/s; 2683773.4 syscalls/s; short_reads=405
buf_size=1024; duration=3.809051675s; num_syscalls=5662564; 1449.7 MiB/s; 1486607.3 syscalls/s; short_reads=15409
buf_size=2048; duration=3.68525519s; num_syscalls=2416487; 1272.6 MiB/s; 655717.7 syscalls/s; short_reads=29849
buf_size=4096; duration=4.371437904s; num_syscalls=1516051; 1339.2 MiB/s; 346808.3 syscalls/s; short_reads=40294
buf_size=8192; duration=2.336217766s; num_syscalls=375118; 1237.5 MiB/s; 160566.4 syscalls/s; short_reads=21637
buf_size=16384; duration=2.766287328s; num_syscalls=225472; 1247.1 MiB/s; 81507.1 syscalls/s; short_reads=27068
buf_size=32768; duration=2.329937586s; num_syscalls=98858; 1285.8 MiB/s; 42429.5 syscalls/s; short_reads=23013
buf_size=65536; duration=1.993754226s; num_syscalls=41473; 1239.0 MiB/s; 20801.5 syscalls/s; short_reads=19601
buf_size=131072; duration=1.955282285s; num_syscalls=25381; 1249.6 MiB/s; 12980.7 syscalls/s; short_reads=19628
buf_size=262144; duration=2.009988546s; num_syscalls=20611; 1273.1 MiB/s; 10254.3 syscalls/s; short_reads=20431
buf_size=524288; duration=1.959299935s; num_syscalls=20253; 1299.7 MiB/s; 10336.9 syscalls/s; short_reads=20245
buf_size=1048576; duration=2.034348116s; num_syscalls=20612; 1259.5 MiB/s; 10132.0 syscalls/s; short_reads=20612
buf_size=2097152; duration=1.988108456s; num_syscalls=20918; 1318.3 MiB/s; 10521.6 syscalls/s; short_reads=20917
buf_size=4194304; duration=1.991821416s; num_syscalls=20695; 1280.5 MiB/s; 10390.0 syscalls/s; short_reads=20695
buf_size=8388608; duration=2.004051336s; num_syscalls=20221; 1270.4 MiB/s; 10090.1 syscalls/s; short_reads=20221
buf_size=16777216; duration=1.968465835s; num_syscalls=19967; 1306.0 MiB/s; 10143.4 syscalls/s; short_reads=19967
```


## /dev/zero

Best = 256 KiB = 42252.9 MiB/s ; Sometimes the best was 128KiB

```
buf_size=1; duration=2.109977484s; num_syscalls=13218770; 6.0 MiB/s; 6264886.8 syscalls/s; short_reads=0
buf_size=2; duration=2.088255464s; num_syscalls=13140604; 12.0 MiB/s; 6292622.8 syscalls/s; short_reads=0
buf_size=4; duration=1.910415235s; num_syscalls=11890607; 23.7 MiB/s; 6224095.6 syscalls/s; short_reads=0
buf_size=8; duration=2.079450314s; num_syscalls=13003902; 47.7 MiB/s; 6253528.6 syscalls/s; short_reads=0
buf_size=16; duration=1.915836995s; num_syscalls=11869437; 94.5 MiB/s; 6195431.6 syscalls/s; short_reads=0
buf_size=32; duration=2.067651574s; num_syscalls=12376238; 182.7 MiB/s; 5985649.7 syscalls/s; short_reads=0
buf_size=64; duration=1.911540996s; num_syscalls=10840109; 346.1 MiB/s; 5670874.5 syscalls/s; short_reads=0
buf_size=128; duration=2.107312124s; num_syscalls=11217050; 649.8 MiB/s; 5322918.2 syscalls/s; short_reads=0
buf_size=256; duration=1.913087075s; num_syscalls=10214505; 1303.5 MiB/s; 5339278.7 syscalls/s; short_reads=0
buf_size=512; duration=1.617278578s; num_syscalls=8319468; 2511.8 MiB/s; 5144115.6 syscalls/s; short_reads=0
buf_size=1024; duration=2.167249854s; num_syscalls=10706639; 4824.4 MiB/s; 4940196.0 syscalls/s; short_reads=0
buf_size=2048; duration=2.076305115s; num_syscalls=10187969; 9583.6 MiB/s; 4906778.4 syscalls/s; short_reads=0
buf_size=4096; duration=2.085769284s; num_syscalls=9229350; 17284.8 MiB/s; 4424914.1 syscalls/s; short_reads=0
buf_size=8192; duration=1.492790756s; num_syscalls=4774410; 24986.8 MiB/s; 3198311.6 syscalls/s; short_reads=0
buf_size=16384; duration=2.037716318s; num_syscalls=4233701; 32463.6 MiB/s; 2077669.5 syscalls/s; short_reads=0
buf_size=32768; duration=2.099627637s; num_syscalls=2358213; 35098.7 MiB/s; 1123157.7 syscalls/s; short_reads=0
buf_size=65536; duration=2.056928797s; num_syscalls=1290989; 39226.8 MiB/s; 627629.4 syscalls/s; short_reads=0
buf_size=131072; duration=1.921920749s; num_syscalls=627471; 40810.1 MiB/s; 326481.2 syscalls/s; short_reads=0
buf_size=262144; duration=1.89363303s; num_syscalls=320047; 42252.9 MiB/s; 169012.2 syscalls/s; short_reads=0
buf_size=524288; duration=1.943629078s; num_syscalls=154950; 39861.0 MiB/s; 79722.0 syscalls/s; short_reads=0
buf_size=1048576; duration=2.063054168s; num_syscalls=75228; 36464.0 MiB/s; 36464.4 syscalls/s; short_reads=0
buf_size=2097152; duration=1.957132768s; num_syscalls=36103; 36893.3 MiB/s; 18446.9 syscalls/s; short_reads=0
buf_size=4194304; duration=1.895898789s; num_syscalls=17473; 36863.7 MiB/s; 9216.2 syscalls/s; short_reads=0
buf_size=8388608; duration=1.942115609s; num_syscalls=8730; 35959.6 MiB/s; 4495.1 syscalls/s; short_reads=0
buf_size=16777216; duration=1.88241312s; num_syscalls=4192; 35627.1 MiB/s; 2226.9 syscalls/s; short_reads=0
```


## /dev/urandom

Best = 16 KiB = 437.6 MiB/s; 8 KiB is very close 430.7 MiB/s

```
buf_size=1; duration=2.084364827s; num_syscalls=5932957; 2.7 MiB/s; 2846410.1 syscalls/s; short_reads=0
buf_size=2; duration=2.091010967s; num_syscalls=5917160; 5.4 MiB/s; 2829808.2 syscalls/s; short_reads=0
buf_size=4; duration=1.919350039s; num_syscalls=5464481; 10.9 MiB/s; 2847047.6 syscalls/s; short_reads=0
buf_size=8; duration=2.081048437s; num_syscalls=5927683; 21.7 MiB/s; 2848411.8 syscalls/s; short_reads=0
buf_size=16; duration=1.938812619s; num_syscalls=5458516; 43.0 MiB/s; 2815391.2 syscalls/s; short_reads=0
buf_size=32; duration=1.948647279s; num_syscalls=5458516; 85.5 MiB/s; 2801182.2 syscalls/s; short_reads=0
buf_size=64; duration=1.91169911s; num_syscalls=4074980; 130.1 MiB/s; 2131601.1 syscalls/s; short_reads=0
buf_size=128; duration=2.080747087s; num_syscalls=3434066; 201.5 MiB/s; 1650400.5 syscalls/s; short_reads=0
buf_size=256; duration=1.948908268s; num_syscalls=2177227; 272.7 MiB/s; 1117152.1 syscalls/s; short_reads=0
buf_size=512; duration=1.87402351s; num_syscalls=1279427; 333.4 MiB/s; 682716.6 syscalls/s; short_reads=0
buf_size=1024; duration=1.930135279s; num_syscalls=757031; 383.0 MiB/s; 392216.5 syscalls/s; short_reads=0
buf_size=2048; duration=1.903523539s; num_syscalls=403682; 414.2 MiB/s; 212070.9 syscalls/s; short_reads=0
buf_size=4096; duration=2.107747077s; num_syscalls=226948; 420.6 MiB/s; 107673.3 syscalls/s; short_reads=0
buf_size=8192; duration=1.922258559s; num_syscalls=105980; 430.7 MiB/s; 55133.1 syscalls/s; short_reads=0
buf_size=16384; duration=1.988237648s; num_syscalls=55685; 437.6 MiB/s; 28007.2 syscalls/s; short_reads=0
buf_size=32768; duration=2.095612847s; num_syscalls=28707; 428.1 MiB/s; 13698.6 syscalls/s; short_reads=0
buf_size=65536; duration=2.058827447s; num_syscalls=14081; 427.4 MiB/s; 6839.3 syscalls/s; short_reads=0
buf_size=131072; duration=1.92853133s; num_syscalls=6588; 427.0 MiB/s; 3416.1 syscalls/s; short_reads=0
buf_size=262144; duration=1.965202888s; num_syscalls=3372; 428.9 MiB/s; 1715.9 syscalls/s; short_reads=0
buf_size=524288; duration=2.035414308s; num_syscalls=1721; 422.6 MiB/s; 845.5 syscalls/s; short_reads=0
buf_size=1048576; duration=1.967525019s; num_syscalls=846; 429.8 MiB/s; 430.0 syscalls/s; short_reads=0
buf_size=2097152; duration=2.004359535s; num_syscalls=435; 433.3 MiB/s; 217.0 syscalls/s; short_reads=0
buf_size=4194304; duration=2.017434091s; num_syscalls=218; 431.5 MiB/s; 108.1 syscalls/s; short_reads=0
buf_size=8388608; duration=2.047173248s; num_syscalls=108; 421.9 MiB/s; 52.8 syscalls/s; short_reads=0
buf_size=16777216; duration=2.01945166s; num_syscalls=54; 426.8 MiB/s; 26.7 syscalls/s; short_reads=0
```
