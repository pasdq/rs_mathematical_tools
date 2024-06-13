# RS Mathematical Tools

>   This is a TUI math tool developed using Rust crossterm, which runs perfectly on both Linux and Windows platforms.

![234729](https://github.com/liueff/rs_mathematical_tools/assets/16551523/8ebe653f-0031-4c5f-af4d-d876e7295074)

When the program starts, it will by default read the .last.txt file in the current directory. If the file does not exist, the program will create a new one. Starting from line 15, the program will display its contents in the TUI interface. You can also specify a text file to read from the command line.

```
./rs_mathematical_tools input.txt
or
rs_mathematical_tools.exe input.txt
```

除了支持基本的数学运算外，它还支持简单的线性方程。

![图片](https://github.com/liueff/rs_mathematical_tools/assets/16551523/0d5e7f62-ca9b-438c-8c1f-fc1c589ea53b)

It supports variable calculations.

![图片](https://github.com/liueff/rs_mathematical_tools/assets/16551523/9ae7dade-a23f-4bad-b0b4-ac46cabe5990)

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
