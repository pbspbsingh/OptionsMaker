import datetime
import re
from datetime import timedelta

import pytz

import config

MY_TIME_ZONE = pytz.timezone(config.TIME_ZONE)


def now() -> datetime.datetime:
    return datetime.datetime.now(MY_TIME_ZONE)


def days_ago(days: int) -> datetime.datetime:
    time = now() - datetime.timedelta(days=days)
    return time.replace(hour=0, minute=0, second=0, microsecond=0)


def parse_duration_string(duration_str: str) -> timedelta:
    """
    Converts a duration string like '15min', '2h', '1d' into a timedelta object.
    """
    match = re.match(r'(\d+)([hmdmin])', duration_str, re.IGNORECASE)
    if match:
        value_str, unit = match.groups()
        value = int(value_str)
        unit = unit.lower()
        if unit == 'min':
            return timedelta(minutes=value)
        elif unit == 'h':
            return timedelta(hours=value)
        elif unit == 'd':
            return timedelta(days=value)
        elif unit == 'm':  # Assuming 'm' means minutes as 'min' is also handled
            return timedelta(minutes=value)
    raise ValueError(f"Invalid duration string '{duration_str}'")


def seconds_to_human(seconds: int) -> str:
    if seconds < 0:
        return "Duration cannot be negative"
    if seconds == 0:
        return "0 seconds"

    # Calculate days, hours, minutes, and remaining seconds
    days = seconds // (24 * 3600)
    seconds %= (24 * 3600)
    hours = seconds // 3600
    seconds %= 3600
    minutes = seconds // 60
    seconds %= 60

    parts = []
    if days > 0:
        parts.append(f"{days}day{'s' if days > 1 else ''}")
    if hours > 0:
        parts.append(f"{hours}hour{'s' if hours > 1 else ''}")
    if minutes > 0:
        parts.append(f"{minutes}min{'s' if minutes > 1 else ''}")
    if seconds > 0 or not parts:  # Include seconds if there are any, or if it's just seconds
        if seconds > 0 or not parts:
            parts.append(f"{seconds}sec")

    return " ".join(parts) if len(parts) > 1 else parts[0]


def options_expiry_range(td: datetime.date = now().date()):
    s = td + timedelta(days=2)
    weekday = s.weekday()
    days_until_friday = (4 - weekday + 7) % 7
    e = s + timedelta(days=days_until_friday)
    return s, e

if __name__ == "__main__":
    for x in range(1, 30, 1):
        today = datetime.date(2025, 6, x)
        start, end = options_expiry_range(today)
        print(today, start, end)
