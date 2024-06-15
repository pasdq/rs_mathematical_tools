# RS Mathematical Tools

> [!TIP]
> This is a mathematical tool (calculator) developed using Rust, featuring a clean and clear TUI interface that runs perfectly on both Linux and Windows platforms.

![图片](https://github.com/pasdq/rs_mathematical_tools/assets/16551523/f0775180-4f41-4314-aebd-aef63bc0330e)

When the program starts, it will by default read the .last.txt file in the current directory. If the file does not exist, the program will create a new one. Starting from line 15, the program will display its contents in the TUI interface.

You can also specify a text file to read from the command line.

```
./rs_mathematical_tools input.txt
or
rs_mathematical_tools.exe input.txt
```

In addition to supporting basic mathematical operations, it also supports simple linear equations.

![图片](https://github.com/liueff/rs_mathematical_tools/assets/16551523/2366a9a9-2595-4d21-a5c4-c921c8c65b29)


It supports variable calculations.

![图片](https://github.com/liueff/rs_mathematical_tools/assets/16551523/07cb2489-c36d-4a8e-a489-cfcd4b985fa9)

You can enter the keyword `rate` to obtain the US dollar exchange rate.

![图片](https://github.com/R6LB/rs_mathematical_tools/assets/16551523/79ab0647-3600-4d6c-bcf6-1450640712ed)


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

> [!NOTE]
> Please refer to the release notes for other features.
