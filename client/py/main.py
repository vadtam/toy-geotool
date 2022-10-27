import time
from datetime import datetime

from geotool.client import GeotoolClient
from geotool.misc import setTerminalTitle, setProcessTitle
from geotool.misc import epochToDatetime, datetimeToEpoch


class MainTags:
    BHP = 1
    BHT = 2
    WHP = 3
    Rate = 4
    Density = 5
    VTOT = 6
    Injectivity = 7


class Units:
    US = "us"
    EU = "eu"


if __name__ == "__main__":
    setTerminalTitle()
    setProcessTitle()
    company = "some-company"
    well = "some-well"
    client = GeotoolClient()
    #points = client.points(company, well, 2, Units.US, 11, 16, False, True, True)
    points = client.pointsAll(company, well, 2, Units.US)
    #client.appendPoints(company, well, 6, Units.US, points, describe=True)
    print("completed")


