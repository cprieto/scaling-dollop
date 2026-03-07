# What is this?

This is mainly a project to learn more about Rust and what make a "Rusty" library that behaves and looks like Rust, while I had been coding Rust for many years now, my way of thinking is still very shaped by object oriented languages like C++, so the purpose is to learn more about how to do libraries and work with Rust in a more natural way.

The motto for this is:

> Don't give me a fish, teach me how to fish!

When teaching me something, explain the Rust concept, the reasoning behind it and maybe some examples. My teaching style is more like:
- Explain *why*, not just what is the fix
- Reference relevant Rust concept (ownership, borrowing, trait, etc...)
- Show a concrete example before/after
- Maybe point to standard library or other existing examples when this is used
- If you notice something not idiomatic, please suggest a change and the reasoning

Please prefer thorough explanations (the _why_ of this, a good example is how Cow allocates memory). Do not over explain the _what_ to change. A good example of this is changing `to_string` and use instead `to_owned` in one implementation using `Cow`:

- Consider what `Cow` does, if the contained object is already in the heap (a `String` for example), `to_owned` will just return that object to you from the heap but if the object lives in the stack, it will need to _copy_ and reallocate.

Maybe use bullet points to explain. When giving code feedback, use numbered suggestions so I can go back to them and explain my reasoning, like a chat with a professor or a senior peer.

## What is the project

I chosed a first simple project, a library to read and write DBF files all based in specifications and pure experimentation.

Good sources for information about DBF structure and files:

1. [XFile Format Description](http://www.manmrk.net/tutorials/database/xbase/#INDEX) this is probably the most complete guide out there
2. [DBF File Format](https://www.dbf2002.com/dbf-file-format.html) from a DBF Viewer and editor, simple to read
3. [DBF and DBT/FPT File Structure](https://www.independent-software.com/dbase-dbf-dbt-file-format.html) is a nice one with some comments and sample

It is important to notice not all of them are actually completely right about all details, I had found discrepancies and that is why I am creating files manually in the original software to use as reference.

## How am I testing reading

For experimentation I had installed DBase3, 4, 5 and DBase 5.5 for Windows as well as FoxPro 1 and FoxPro 2 in a DOSBox-x machine. For Visual FoxPro I installed Visual FoxPro in a 86Box virtual machine. I created common files to try and parse them, the structure is simple:

For DBase3:

| Field name | Type      | Width | Decimal |
| ---------- | --------- | ----- | ------- |
| NAME       | Character | 20    | 0       |
| PRICE      | Numeric   | 10    | 2       |
| QTY        | Numeric   | 6     | 0       |
| ACTIVE     | Logical   | 1     | 0       |
| ADDED      | Date      | 8     | 0       |

For DBase 4/5 and FoxPro 1/2 they support additional type `FLOAT` so this field was added

| Field name | Type  | Width | Decimal |
| ---------- | ----- | ----- | ------- |
| WEIGHT     | Float | 8     | 0       |

And Visual FoxPro supports new types: `Integer`, `Double`, `Currency` and `DateTime`

| Field name | Type     | Width | Decimal |
| ---------- | -------- | ----- | ------- |
| MARGIN     | Double   | 8     | 4       |
| UPDATED    | DateTime | 8     | 0       |
| COST       | Currency | 8     | 0       |

Files for those tests and samples to compare are named `db3.dbf`, `db4.dbf`, `db5.dbf`, `fox1.dbf`, `fox2.dbf` and `vfp.dbf`

Additionally, some files exists to check memo support: `db3memo.dbf`, `db4memo.dbf`, `db5memo.dbf`, `fox1memo.dbf`, `fox2memo.dbf`, `vfpmemo.dbf` and each of them has their memo contrapart with extension `dbt` for DBase files and `fpt` for FoxPro files.

Memo structure is simple:

| Field name | Type      | Length |
| ---------- | --------- | ------ |
| TITLE      | Character | 20     |
| NOTES      | Memo      | 10     |

## What about writing?

I am not supporting writing or indexing yet, it is another milestone, so for now do not suggest anything related with writers or using indices.

## Good comparisson projects

There are already projects to handle databases (and DBF are database files), for example [sqlx](https://docs.rs/sqlx/latest/sqlx/) so this could be a good guide for it. Please avoid suggesting things from other DBF reading projects, if we converge it should be by pure random event, I want to get into those conclusions about how the API and library looks by myself (with your help and guide of course).

## Next tasks

- [ ] Time to read rows

claude --resume 3e5c9565-f505-4fe2-b307-f3a760360b4d
