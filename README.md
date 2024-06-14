# RS Mathematical Tools

>   This is a TUI mathematical tools (calculations) developed using Rust crossterm, which runs perfectly on both Linux and Windows platforms.

![图片](https://github.com/R6LB/rs_mathematical_tools/assets/16551523/4453a36d-a804-41c8-a279-254fa8b187d5)


When the program starts, it will by default read the .last.txt file in the current directory. If the file does not exist, the program will create a new one. Starting from line 15, the program will display its contents in the TUI interface. You can also specify a text file to read from the command line.

```
./rs_mathematical_tools input.txt
or
rs_mathematical_tools.exe input.txt
```

In addition to supporting basic mathematical operations, it also supports simple linear equations.

![图片](https://github.com/liueff/rs_mathematical_tools/assets/16551523/2366a9a9-2595-4d21-a5c4-c921c8c65b29)


It supports variable calculations.

![图片](https://github.com/liueff/rs_mathematical_tools/assets/16551523/07cb2489-c36d-4a8e-a489-cfcd4b985fa9)


It supports cursor movement using the up and down arrow keys and the Tab key, as well as mouse operations. Additionally, it has several shortcut keys.

```
CTRL+T MOVE TO THE TOP
CTRL+B MOVE TO THE BOTTOM
CTRL+A MOVE TO THE HEADER
CTRL+E MOVE TO THE END

CTRL+L CLEAR CURRENT
CTRL+U CLEAR ALL

F4 OPEN & CLOSE INPUT
F5 RECALCULATE AND SAVE
CTRL+C SAVE & EXIT
```
