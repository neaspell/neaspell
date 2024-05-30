The dictionary for one language consists of either two files or one file.
* Language rules are in the "aff" file and words and in the "dic" file.
* Both rules and word are in the "neadic" file.

The neadic format always uses UTF-8 encoding without BOM.
The aff file can use different encodings as defined by SET element.

Elements by functionality

* PFX, SFX, WORDCHARS, SET, FLAG, DIC, NEA
* REP, TRY, KEY

Elements in alphabetic order:
SET
TRY
WORDCHARS

# NEA

NEA COMPAT
This is the first non-blank line of neadic file.
