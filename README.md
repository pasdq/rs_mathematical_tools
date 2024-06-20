# RS Mathematical Tools

> [!TIP]
> This is a mathematical tool (calculator) developed using Rust, featuring a clean and clear TUI interface that runs perfectly on both Linux and Windows platforms.

![图片](https://github.com/pasdq/rs_mathematical_tools/assets/16551523/878095ed-534f-473e-a8c7-bcc7e4e5dabc)

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

    Navigation:
        ↑ and ↓: Move between input fields.
        ← and →: Move within an input field.
        PageUp and PageDown: Switch between sections.
    Editing:
        Enter: Evaluate the current expression or execute a command.
        Ctrl + U: Clear all inputs.
        Ctrl + L: Clear the current input.
        Backspace: Delete the character to the left of the cursor.
    Miscellaneous:
        Ctrl + C: Exit the program.
        F4: Toggle between locked and unlocked status.
        F5: Save current inputs to the file.

**Commands**

    Function Commands (fc.*): Load predefined inputs from the .func.toml file.
        Example: fc.section_name
    Constant Commands (cst.*): Insert predefined constant values.
        Example: cst.item_name
    Special Commands:
        about: Display information about the program.
        rate: Execute an external rate calculation command.

**Customization**

Customize the TUI by setting the color and attribute fields in the [TUI] section.
Supported Colors

    Blue, Red, Green, Yellow, Magenta, Cyan, White, Black, DarkRed, DarkGreen, DarkYellow, DarkBlue, DarkMagenta, DarkCyan, Grey, DarkGrey


Supported Attributes

    Bold, Underlined, Reverse, NoBold, NoUnderline, NoReverse, Italic, NoItalic, Dim, NormalIntensity, SlowBlink, RapidBlink, NoBlink, Hidden, NoHidden, CrossedOut, NotCrossedOut


> [!NOTE]
> For any questions or suggestions, please open an issue on the GitHub repository. Enjoy using RS Mathematical Tools for your mathematical computations!
