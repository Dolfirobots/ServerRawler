---
sidebar_position: 7
---

# `--convert-image`

**Long**: `--convert-image`

Converts a Base64 encoded image string (often found in Minecraft server favicons as a Data URI) into a physical image file. This utility is useful for extracting and saving server icons.

## Usage

```bash
ServerRawler --convert-image <FILE_PATH> <BASE64_STRING>
```

* `<FILE_PATH>`: The path where the image file should be saved (e.g., `favicon.png`). The file extension will determine the output format.
* `<BASE64_STRING>`: The full Base64 encoded string, optionally including the `data:image/<type>;base64,` prefix.

## Examples

To convert a Base64 string to a PNG image:

```bash
ServerRawler --convert-image favicon.png "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA..."
```

To convert a Base64 string without the Data URI prefix:

```bash
ServerRawler --convert-image image.jpg "iVBORw0KGgoAAAANSUhEUgAA..."
```