from datetime import datetime
from setproctitle import setproctitle  # pip3 install setproctitle


def setTerminalTitle():
    title = "GEOTOOL CLIENT"
    print(f'\33]0;{title}\a', end='', flush=True)


def setProcessTitle():
    setproctitle("geotool-python-client")


def epochToDatetime(epoch):
    '''
    returns datetime, local time

    NB: if you create a datetime object using datetime,
       e.g. datetime(2018, 8, 1)
       and then call datetimeToEpoch, then epochToDatetime,
       you will get the same datetime. I.e. localtime zone is used by default.

    arg: epoch is seconds, int
    '''
    return datetime.fromtimestamp(epoch)


def datetimeToEpoch(dt):
    '''
    returns epoch in seconds, int

    arg: dt is datetime, UTC time.
      if your timezone is not timezone-aware, make the corresponding time correction.
    
    use this function sensibly, test before use.
    '''
    return int(dt.timestamp())


