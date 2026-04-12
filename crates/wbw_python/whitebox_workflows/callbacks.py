from __future__ import annotations

import json
import re
import sys
from typing import Any, TextIO


_PERCENT_IN_MESSAGE_RE = re.compile(r"(-?\d+(?:\.\d+)?)\s*%")


def _normalize_event(event: Any) -> tuple[str | None, Any, str | None] | None:
    parsed = event
    if isinstance(event, str):
        try:
            parsed = json.loads(event)
        except json.JSONDecodeError:
            return (None, None, event)

    if isinstance(parsed, dict):
        return (
            parsed.get("type"),
            parsed.get("percent"),
            parsed.get("message"),
        )

    return (
        getattr(parsed, "type", None),
        getattr(parsed, "percent", None),
        getattr(parsed, "message", None),
    )


class ProgressPrinter:
    """Stateful callback that prints integer progress updates and messages.

    The callback accepts event payloads as JSON strings, dictionaries, or objects
    exposing ``type``, ``percent``, and ``message`` attributes.
    """

    def __init__(
        self,
        *,
        show_messages: bool = True,
        min_increment: int = 1,
        stream: TextIO | None = None,
    ) -> None:
        self.show_messages = show_messages
        self.min_increment = max(1, int(min_increment))
        self.stream = stream if stream is not None else sys.stdout
        self._last_reported_percent = -1

    def reset(self) -> None:
        """Reset internal progress state for a fresh run."""
        self._last_reported_percent = -1

    def __call__(self, event: Any) -> None:
        normalized = _normalize_event(event)
        if normalized is None:
            if self.show_messages:
                print(event, file=self.stream)
            return

        event_type, raw_percent, message = normalized

        if event_type != "progress" or raw_percent is None:
            if raw_percent is None and message:
                inferred = _infer_percent_from_message(message)
                if inferred is not None:
                    self._emit_percent(inferred)
                    if self.show_messages:
                        print(message, file=self.stream)
                    return

            if self.show_messages:
                if message:
                    print(message, file=self.stream)
                elif event_type and event_type != "progress":
                    print(str(event_type), file=self.stream)
                else:
                    print(event, file=self.stream)
            return

        try:
            percent = float(raw_percent)
        except (TypeError, ValueError):
            if self.show_messages and message:
                print(message, file=self.stream)
            return

        self._emit_percent(percent)

    def _emit_percent(self, raw_percent: float) -> None:
        percent = float(raw_percent)
        if percent <= 1.0:
            percent *= 100.0
        percent_int = max(0, min(100, int(percent)))

        # If progress decreases, treat it as a new tool invocation.
        if percent_int < self._last_reported_percent:
            self._last_reported_percent = -1

        should_print = (
            percent_int >= self._last_reported_percent + self.min_increment
            or percent_int == 100
        )
        if should_print and percent_int > self._last_reported_percent:
            print(f"{percent_int}%", file=self.stream)
            self._last_reported_percent = percent_int


def _infer_percent_from_message(message: Any) -> float | None:
    text = str(message)
    match = _PERCENT_IN_MESSAGE_RE.search(text)
    if not match:
        return None
    try:
        return float(match.group(1))
    except ValueError:
        return None


def make_progress_printer(
    *,
    show_messages: bool = True,
    min_increment: int = 1,
    stream: TextIO | None = None,
) -> ProgressPrinter:
    """Create a configurable progress-printing callback."""
    return ProgressPrinter(
        show_messages=show_messages,
        min_increment=min_increment,
        stream=stream,
    )


_default_progress_printer = ProgressPrinter()


def print_progress(event: Any) -> None:
    """Built-in progress callback with sensible defaults.

    Intended usage:

    ``result = wbe.raster.abs(input=dem, callback=wb.callbacks.print_progress)``

    This callback always writes progress messages to stdout. ``wbe.verbose`` only
    controls environment/runtime status messages emitted by the bindings; it does
    not disable an explicit callback printer. Pass ``callback=None`` for silent
    execution.
    """
    _default_progress_printer(event)


__all__ = ["ProgressPrinter", "make_progress_printer", "print_progress"]
