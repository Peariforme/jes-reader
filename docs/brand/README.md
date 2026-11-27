# Brand assets

Sources of the project's images. Render them again after a change:

```sh
magick -background none -density 300 docs/brand/icon.svg -resize 256x256 editors/vscode/images/icon.png
magick -background none -density 300 docs/brand/icon.svg -resize 64x64 docs/theme/favicon.png
cp docs/brand/icon.svg docs/theme/favicon.svg
magick -background none -density 96 docs/brand/social-preview.svg docs/src/images/social-preview.png
```

- `icon.svg`: extension icon, README logo and documentation favicon. A raw sysout becomes a structured list, colored by severity.
- `social-preview.svg`: 1280×640 card for GitHub (*Settings → Social preview*), LinkedIn and Slack. Its text uses the Consolas and Segoe UI fonts: render it on Windows, or the fallbacks change its look.
