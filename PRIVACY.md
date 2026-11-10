# Privacy

Browser Picker has no account, no telemetry, no analytics and no advertising.
It does not collect or send any information about you or the links you click.

## What stays on your PC

- Your rules: `%LOCALAPPDATA%\BrowserPicker\rules.json`
- Your settings: `settings.json` in the same folder
- Chrome and Edge profile names and photos are read from those browsers' own
  local files to draw the picker. They are not copied or sent anywhere.

## When the app uses the network

There are exactly two cases.

1. **Update check.** When the rules manager is open, an installed build asks
   GitHub Releases (`github.com/fredle/browser-picker`) whether a newer
   version exists, and downloads it if you press *Update & restart*. GitHub
   sees your IP address, as with any download. No link or rule data is sent.

2. **Protected links - off by default.** If you tick the option in the rules
   manager, a link hosted on a known email safe-link service (Mimecast,
   Microsoft SafeLinks, Proofpoint, Barracuda, Symantec) is requested from
   that service to learn its real destination. The service will record this
   as a click on the original link. The destination page itself is not
   downloaded. Ordinary links never trigger a request.

Clicking a link with no rule and no picker interaction beyond choosing a
profile makes no network request from Browser Picker at all; the link is
handed to the browser you chose.

## Contact

Questions or concerns: https://github.com/fredle/browser-picker/issues
