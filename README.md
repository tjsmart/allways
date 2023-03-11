allways
======================

Automatically update `__all__` statements.

## Installation

```bash
pip install allways
```


## Command line interface

```bash
allways <file1.py> <file2.py> ...
```

## As a pre-commit hook

See [pre-commit](https://github.com/pre-commit/pre-commit) for instructions.

Sample `.pre-commit-config.yaml`

```yaml
-   repo: https://github.com/tjsmart/allways
    rev: v0.0.1
    hooks:
    -   id: allways
```

Note: by default the pre-commit hook will run only against `__init__.py` files.

## What does it do?

### Add `__all__` statements to your python files

```python
from ._foo import bar
from ._x import y as z

def foo():
    ...
```

becomes

```python
from ._foo import bar
from ._x import y as z

def foo():
    ...


# allways: start
__all__ = [
    "bar",
    "foo",
    "z",
]
# allways: end
```

### Ignore private variables

```python
from . import _foo
from . import bar
```

becomes

```python
from . import _foo
from . import bar


# allways: start
__all__ = [
    "bar",
]
# allways: end
```

### Update pre-existing `__all__` statements

```python
from . import bar
from . import baz


# allways: start
__all__ = [
    "bar",
    "foo",
]
# allways: end
```

becomes

```python
from . import bar
from . import baz


# allways: start
__all__ = [
    "bar",
    "baz",
]
# allways: end
```

## Why?

### the problem

I choose to organize python libraries with:
- [PEP 561](https://peps.python.org/pep-0561/#packaging-type-information) compliance
- private module files, public (sub-)package  folders
- using `__init__.py` to define the public interface of a (sub-)package

For example, I might layout my project as such:
```
pkg/
├── bar/
│   ├── _bar.py
│   └── __init__.py
├── _foo.py
├── __init__.py
└── py.typed
```

Contained in the files `pkg/_foo.py` and `pkg/bar/_bar.py` there will be some portion that I would like to expose publicly via `pkg/__init__.py` and `pkg/bar/__init__.py`, respectively.

For example, perhaps I would like to expose a function `do_something` from the module file `pkg/_foo.py` by adding the following line to `pkg/__init__.py`:
```python
from ._foo import do_something
```

***And here is where the problem arises!*** (I *know*... a lot of setup...)

When a user of our package turns to use `do_something` they will be slapped on the wrist by the type-checker.

- `pyright` output:
```
t.py:1:18 - error: "do_something" is not exported from module "pkg"
Import from "pkg._foo" instead (reportPrivateImportUsage)
```

- `mypy --strict` output:
```
t.py:1: error: Module "pkg" does not explicitly export attribute "do_something"  [attr-defined] 
```

And if you aren't concerned that users will have to ignore this type error, know that it get's worse! Language servers will not display any hint that the object `pkg.do_something` exists. 😱

For small projects maintaining this by hand is no big deal. But for large projects with several contributors this becomes a complete wast of time! 😠


### the solution

According to [pyright documentation](https://github.com/microsoft/pyright/blob/main/docs/typed-libraries.md#library-interface), a typed library can choose to explicitly re-export symbols by adding it to the `__all__` of the corresponding module.

`allways` mission is to automate the process of updating `__all__` statements in `__init__.py` files. 🤗

### but also

My personal goal here is to contribute something to open source and write some more rust! 🦀