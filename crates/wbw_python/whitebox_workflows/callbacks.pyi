from __future__ import annotations

from typing import Any, TextIO


class ProgressPrinter:
    show_messages: bool
    min_increment: int
    stream: TextIO
    def __init__(self, *, show_messages: bool = True, min_increment: int = 1, stream: TextIO | None = None) -> None: ...
    def reset(self) -> None: ...
    def __call__(self, event: Any) -> None: ...


def make_progress_printer(*, show_messages: bool = True, min_increment: int = 1, stream: TextIO | None = None) -> ProgressPrinter: ...

def print_progress(event: Any) -> None: ...
