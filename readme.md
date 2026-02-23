# DBF is not dead!

Well, in reality is super dead! I was bored one weekend and decided to write a library to parse/write DBF files, yes those pesky files made by [DBase](https://en.wikipedia.org/wiki/DBase) and [FoxPro](https://en.wikipedia.org/wiki/FoxPro) which were very popular during the 90's for simple database applications.

## What can do so far?

Well, right now only reading memo files (for DBase3, 4 and FoxPro).

## Things next

- [X] Check Visual FoxPro memo files
- [X] Check DBase 5 memo files
- [ ] Move DBT/FPT tests so they use the same memo files for other tests
- [ ] Start parsing DBF!

## Sources

There are a lot of places with different descriptions for DBF files:

1. [XFile Format Description](http://www.manmrk.net/tutorials/database/xbase/#INDEX) this is probably the most complete guide out there
2. [DBF File Format](https://www.dbf2002.com/dbf-file-format.html) from a DBF Viewer and editor, simple to read
3. [DBF and DBT/FPT File Structure](https://www.independent-software.com/dbase-dbf-dbt-file-format.html) is a nice one with some comments and sample code

## Surprises

During my experiments and classic "generating a file, look at its structure" I found a few inconsistencies not appearing _anywhere_, or sometimes, contradicting the file specification posted by someone else. Here I list of those "gotcha" moments

- *Every file without memo is a DBase 3 file*: yep, there is no simple way from the header to see if the file is DBase3, 4, 5, or even FoxPro 1 or 2, all of them are marked by 0x03 which means is treated as a DBase3 file. This gives us some headache and we have to be permissive during reading because field types (yes, there are field types in 4 not in 3, for example `Float`). I will figure out later if I code a writer how would I solve this. For files with memo fields at least we can check wich version is it.
- *Field names are 11 characters, but not all the formats zero-fill it*: Field names are limited to 11 characters and in DBase their names are zero-fill so we can just read 10 bytes and assemble a string from it. For FoxPro, field names are zero-terminated, this means we can contain garbage in a field name so we have to grab those 11 characters and read until we find a 0, not a big deal but this is not documented _anywhere_.

## Future (nice things I will probably do)

This project is mostly for learning things and keep myself busy, so as every hobby I could do or not things as I go along

- Support writing DBF, and DBT/FPT files
- Read and handle indices? (CDX, NDX, MDX)
- Any other crazy idea
