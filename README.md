# SMAAck

A small portable Windows utility for applying SMAA (Subpixel Morphological Antialiasing) to existing images.  Useful for post-processing screenshots or renders from video games or other 3D applications which don't support injecting SMAA with ReShade.  Output images are in lossless WEBP format.

## Usage

1. Place `SMAAck.exe` in a folder containing your images.
2. Double-click it.
3. Wait for processing to finish.
4. The completed images will be placed in a new "Output" subfolder.

No installation or configuration required.

## Examples
<img width="693" height="422" alt="FC4 before" src="https://github.com/user-attachments/assets/a535dd98-d43c-439e-9445-09aad14cf411" />
<img width="693" height="422" alt="FC4 after" src="https://github.com/user-attachments/assets/34bff0a9-e418-40bd-8916-281a677a296b" />
<img width="861" height="701" alt="COD1 before" src="https://github.com/user-attachments/assets/ef1a20f4-de9b-495a-a70c-cf10abce9963" />
<img width="861" height="701" alt="COD1 after" src="https://github.com/user-attachments/assets/285f9a1a-0a19-4304-915c-ce1cbae7de61" />
<img width="1504" height="1272" alt="Spo3 before" src="https://github.com/user-attachments/assets/ea91a3b5-2d4e-4c2c-a1e8-002743f1fcb9" />
<img width="1504" height="1272" alt="Spo3 after" src="https://github.com/user-attachments/assets/18def58f-352c-4b9c-a6da-2c4088beeb35" />

## Supported input formats

PNG, JPG, WEBP, BMP

## Requirements

Requires a GPU with a graphics API supported by wgpu.  Tested on Windows 10.


This is a small personal utility rather than a polished, professional software project. It was created to solve a specific problem with preexisting images, and intentionally has no GUI or settings to be as simple as possible.

## Acknowledgements
Developed with assistance from Anthropic's Claude Opus 4.8.
