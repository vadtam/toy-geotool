from geotool.misc import epochToDatetime


class Point:
    def __init__(self, time, value):
        self.time = int(time)      # epoch time, seconds
        self.value = float(value)  # must be float
        if self.time <= 0:
            raise ValueError ("Epoch time must be positive!")
        elif self.time > 2147483646:
            raise ValueError ("Epoch time must be in seconds, not milliseconds!")

    def __str__(self):
        return ("(local timezone) point: " + str(epochToDatetime(self.time)) + ", " +
           str(self.value))

    def toJSON(self):
        return {"time": self.time, "value": self.value}

    @classmethod
    def fromJSON(cls, row):
        return cls(row["time"], row["value"])


