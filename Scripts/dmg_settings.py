# dmgbuild settings for the Yap installer DMG.
# Invoked by Scripts/make-dmg.sh:  dmgbuild -s Scripts/dmg_settings.py "Yap" out.dmg
import os.path

app = os.environ.get("YAP_APP", "build/Yap.app")
appname = os.path.basename(app)

# Contents
files = [app]
symlinks = {"Applications": "/Applications"}
badge_icon = os.environ.get("YAP_ICNS", "Bundle/AppIcon.icns")

# Appearance
background = os.environ.get("YAP_DMG_BG", "build/dmg-bg.png")
format = "UDZO"
default_view = "icon-view"
icon_size = 128
window_rect = ((200, 200), (640, 400))
icon_locations = {
    appname: (160, 205),
    "Applications": (480, 205),
}
