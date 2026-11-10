# Microsoft Store listing

Source of truth for the Store listing text. Edit here first, then copy into
Partner Center (or hand `partner-center-prompt.md` to a browser agent). Every
claim below should stay true of the shipped build - in particular the privacy
paragraph must keep matching `PRIVACY.md`.

Each fenced block is one Partner Center field, pasted verbatim.

## Product name

Reserve it first under Product management -> Manage app names. The installed
app keeps the short name "Browser Picker".

```text
Browser Picker: Chrome & Edge Profile Switcher
```

## Short description

```text
Work link opened in your personal Chrome profile again? Browser Picker asks which Chrome or Edge profile should open each link you click, or remembers your choice for that site so you are never asked twice. Fast, tiny, and nothing runs in the background.
```

## Description

```text
Work link opened in your personal profile again? Browser Picker fixes that.

Set Browser Picker as your default browser and every link you click in Outlook, Teams, Slack or any other app shows a small picker with your Chrome and Edge profiles, each with its real name and photo. Click one, or press its number, and the link opens there.

SAVE A RULE, SKIP THE PICKER
Right-click a profile to make it the default for that site. From then on, links to that site open straight in the right profile with no window at all. Rules support wildcards, so one rule can cover a whole company domain. Hold Shift while clicking a link to ignore your rules and get the picker anyway.

BUILT FOR PEOPLE WITH MORE THAN ONE PROFILE
• Work and personal accounts kept apart
• Consultants and IT admins signed in to several client tenants
• Shared family PCs where each person has their own profile
• Incognito and InPrivate available as choices, and as rule targets

SEES THROUGH PROTECTED EMAIL LINKS
Links rewritten by Mimecast, Microsoft Defender SafeLinks, Proofpoint, Barracuda or Symantec all look the same to a rule. Turn on the optional setting and Browser Picker resolves them to the real destination first, so the picker shows where the link actually goes and your rules match the real site.

FAST AND OUT OF THE WAY
• The picker appears in about a quarter of a second
• A matched rule opens your browser with no window shown
• No background process, no tray icon, nothing running when you are not clicking a link
• A single small program of about 5 MB with nothing else to install
• Opens on the monitor your cursor is on

EASY TO MANAGE
• See, edit and delete your rules in one window
• A setup guide shows whether Browser Picker is your default browser and takes you to the right Windows settings page if it is not
• Installs for your user account only, with no administrator rights
• Signed, and updates itself

PRIVATE BY DESIGN
Your rules are stored on your PC and never leave it. There is no account, no telemetry and no advertising. The only network use is checking for app updates and, if you switch it on, resolving protected email links.

Works with Google Chrome and Microsoft Edge.
```

## What's new in this version

Trim to what has shipped since the version currently on the Store.

```text
• Browser Picker now updates itself: an "Update & restart" button appears in the rules manager when a new version is ready
• New optional setting to resolve protected email links (Mimecast, SafeLinks, Proofpoint and others) to their real destination
• After installing, you are guided to set Browser Picker as your default browser
• The running version is shown in the rules manager
```

## Product features

One per line, in this order.

```text
Choose which Chrome or Edge profile opens every link you click
Profiles shown with their real names and photos
Save a rule so a site always opens in the same profile
Wildcard rules to cover a whole domain
Press 1 to 9 to pick a profile from the keyboard
Hold Shift while clicking to bypass your rules
Resolves Mimecast, SafeLinks, Proofpoint and Barracuda links to the real site
Incognito and InPrivate as picker choices
No background process and no tray icon
Picker appears in about a quarter of a second
Installs without administrator rights
Updates itself
Rules stay on your PC, with no account and no telemetry
```

## Search terms

```text
chrome profile
edge profile
default browser
browser chooser
browser switcher
open link in profile
safelinks
```

## Screenshot captions

Screenshots themselves have to be captured on Windows; these are the shots
and their captions, in display order.

| # | Shot | Caption |
|---|---|---|
| 1 | Picker over an Outlook or Teams link, real profile avatars | Pick the right Chrome or Edge profile for every link |
| 2 | Right-click menu on a profile card | Right-click to always open a site in the same profile |
| 3 | Rules manager with a few rules | All your rules in one place |
| 4 | Rules manager with the protected-links checkbox ticked | See where protected email links really go |
| 5 | Setup banner showing default-browser status | Guided setup, no administrator rights needed |

## Other fields

| Field | Value |
|---|---|
| Category | Utilities & tools |
| Website | https://github.com/fredle/browser-picker |
| Support contact | https://github.com/fredle/browser-picker/issues |
| Privacy policy URL | https://github.com/fredle/browser-picker/blob/main/PRIVACY.md |
| Minimum requirements | Google Chrome or Microsoft Edge installed |
