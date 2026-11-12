# Fonts

`ScorchMono.ttf` is Terminus (TTF) 4.49.3 by Dimitar Zhekov and Tilman Blumenbach, modified for the game Scorch and
shared by all games here: the Cyrillic letters в and д use Terminus' own alternative variant `dv1` (standard print
shapes instead of the default handwritten-style ones), and и and й are redrawn with the standard diagonal (Terminus
draws them like a Latin u at this size). Embedded bitmaps were removed, so all sizes use the outlines.

The SIL Open Font License (`LICENSE-OFL.txt`) reserves the names "Terminus Font" and "Terminus (TTF)", so the modified
font is named "Scorch Mono". The script that builds it, `tools/patch_font.py`, is in the Scorch repository.

The font has no emoji. Before using a new symbol, check the font has it: `fc-query gamekit/assets/fonts/ScorchMono.ttf`.
