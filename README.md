# rust-srcprof

This is reimplementation of source code line count profile I wrote long time ago in Python.

As it is implemented in Rust, it should be much faster than Python version, especially with
large repository.

Of course, number of lines is a stupid metric to evaluate a code base in any standard,
but still it gives some amount of insight if you have no knowledge at all about it.

## Example output

It shows summary of lines for each file extension, top `n` largest files and distribution in semi-log plot.

```

--------------------------
     Summary
--------------------------

"c": files = 40, lines = 8460, size = 254453
"cpp": files = 174, lines = 98848, size = 2928898
"sh": files = 2, lines = 14, size = 439
"rc": files = 2, lines = 262, size = 11470
"h": files = 231, lines = 29704, size = 1125623
"py": files = 2, lines = 217, size = 6797
total: files = 451, lines = 137505, size = 4327680

--------------------------
      Top 10 files
--------------------------

6495: D:\projects\gltestplus\clib\include\GL\glext.h
3158: D:\projects\gltestplus\mods\vastspace\island3.cpp
2771: D:\projects\gltestplus\src\gltestplus.cpp
2571: D:\projects\gltestplus\mods\surface\DrawMap.cpp
2228: D:\projects\gltestplus\src\draw\DrawTextureSphere.cpp
2142: D:\projects\gltestplus\src\glw\glwindow.cpp
2077: D:\projects\gltestplus\mods\vastspace\Soldier.cpp
1865: D:\projects\gltestplus\src\astrodraw.cpp
1846: D:\projects\gltestplus\src\CoordSys.cpp
1837: D:\projects\gltestplus\src\Sceptor.cpp

--------------------------
      Distribution
--------------------------

    1-    1   0: 
    2-    1   0: 
    2-    3   1: *
    4-    4   0: 
    5-    7   7: ********
    8-   10  12: ***************
   11-   15  19: ************************
   16-   21  12: ***************
   22-   31  21: **************************
   32-   44  35: ********************************************
   45-   63  40: ***************************************************
   64-   89  37: ***********************************************
   90-  127  43: ******************************************************
  128-  180  47: ************************************************************
  181-  255  35: ********************************************
  256-  361  32: ****************************************
  362-  511  34: *******************************************
  512-  723  32: ****************************************
  724- 1023  11: **************
 1024- 1447  15: *******************
 1448- 2047  10: ************
 2048- 2895   5: ******
 2896- 4095   1: *
 4096- 5791   0: 
 5792- 8191   1: *
 8192-11584   0: 
11585-16383   0: 
16384-23169   0: 
23170-32767   0: 
32768-46339   0: 
```

## Prerequisites

* Cargo 1.56.0

## How to run

   cargo run [options] <path>

Full list of options can be obtained by `cargo run -- --help`.
