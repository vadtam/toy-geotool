import os
import time
import threading
import requests
import urllib3
import json
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

from datetime import datetime

from geotool.point import Point


class GeotoolClient:
    BATCH_SIZE = 9000  # split request into subrequests

    def __init__(self, debug=False):
      self.username = os.environ['GEOTOOL_CLIENT_USERNAME']
      self.password = os.environ['GEOTOOL_CLIENT_PASSWORD']
      self.debug = debug
      if self.debug:
          self.host = "https://localhost:8001"
          self.isRequestVerificationEnabled = False
      else:
          self.host = "https://geotool.cloud"
          self.isRequestVerificationEnabled = True

      self.session = requests.Session()
      self.setupConnection()
      self.launchPingThread()

    def setupConnection(self):
        payload = {
            "username": self.username,
            "password": self.password,
            "nextUrl" : "/"
        }
        r = self.session.post(self.host + "/login", data=payload,
                verify=self.isRequestVerificationEnabled)
        if r.status_code == 202:
            if "token" not in self.session.cookies.get_dict(): 
                raise ValueError("bad credentials")
        elif r.status_code == 503:
            raise ConnectionError("server down.")
        else:
            raise ValueError

    def launchPingThread(self):
        def pingServer():
            self.session.get(
                self.host + "/api/ping",
                verify=self.isRequestVerificationEnabled)

        def pingServerLoop():
            while(True):
               time.sleep(60*10)
               pingServer()

        x = threading.Thread(target=pingServerLoop, daemon=True)
        x.start()

    def getTagUrl(self, company, well, tag):
        if type(tag) is not int:
            raise TypeError
        ss = (self.host + "/companies/" + company + "/wells/" + well +
                "/tags/" + str(tag))
        return ss

    def isShortTag(self, company, well, tag):
        val = abs(tag)
        if val == 0:
            raise ValueError
        if val <= 7:
            if val == 6:
                return False
            else:
                return True
        else:
            tagUrl = self.getTagUrl(company, well, abs(tag))
            r = self.session.get(tagUrl + "/is-f32",
                    verify=self.isRequestVerificationEnabled)
            if r.status_code == 404:
                raise ValueError("this custom tag does not exist")
            if r.text == "false":
                return False
            else:
                return True

    def lastPoint(self, company, well, tag, units):
        '''
        returns Point or None
        '''
        tagUrl = self.getTagUrl(company, well, tag)
        payload = {
            "units": units
        }
        if self.isShortTag(company, well, tag):
            reqUrl = tagUrl + "/last-point-f32"
        else:
            reqUrl = tagUrl + "/last-point-f64"
        r = self.session.post(reqUrl, data=payload,
                              verify=self.isRequestVerificationEnabled)
        body = r.json()
        if body is None:
            return None
        else:
            time = int(body['time'])
            value = float(body['value'])
            return Point(time, value)

    def firstPoint(self, company, well, tag, units):
        '''
        returns Point or None
        '''
        tagUrl = self.getTagUrl(company, well, tag)
        payload = {
            "units": units
        }
        if self.isShortTag(company, well, tag):
            reqUrl = tagUrl + "/first-point-f32"
        else:
            reqUrl = tagUrl + "/first-point-f64"
        r = self.session.post(reqUrl, data=payload,
                              verify=self.isRequestVerificationEnabled)
        body = r.json()
        if body is None:
            return None
        else:
            time = int(body['time'])
            value = float(body['value'])
            return Point(time, value)

    def points(self, company, well, tag, units, timeFrom, timeTo, LBS, RBS, IPB):
        '''
        returns []Point

        args:
            timeFrom, timeTo: epoch seconds, int32
            time = 0 means open boundary
            LBS - left boundary strictness, bool
            RBS - right boundary strictness, bool
            IPB - include point before, bool
        '''
        if type(timeFrom) is not int:
            raise TypeError
        if type(timeTo) is not int:
            raise TypeError
        tagUrl = self.getTagUrl(company, well, tag)
        payload = {
            "units": units,
            "timeFrom": timeFrom,
            "timeTo": timeTo,
            "LBS": LBS,
            "RBS": RBS,
            "IPB": IPB,
        }
        if self.isShortTag(company, well, tag):
            reqUrl = tagUrl + "/points-f32"
        else:
            reqUrl = tagUrl + "/points-f64"
        r = self.session.post(reqUrl, data=payload, stream=True,
                              verify=self.isRequestVerificationEnabled)
        points = []
        for line in r.iter_lines():
            if line:
                row = json.loads(line)
                point = Point.fromJSON(row)
                points.append(point)
        #
        return points

    def pointsAll(self, company, well, tag, units):
        return self.points(company, well, tag, units, 0, 0, False, False, False)

    def appendPoints(self, company, well, tag, units, points, describe=False):
        if len(points) == 0:
            return
        tagUrl = self.getTagUrl(company, well, tag)
        if self.isShortTag(company, well, tag):
            reqUrl = tagUrl + "/points-f32-append"
        else:
            reqUrl = tagUrl + "/points-f64-append"

        STEP = 27000
        batches = [points[x:x+STEP] for x in range(0, len(points), STEP)]
        if describe:
            len_batches = len(batches)
            print("(decribe appendPoints) there are " + str(len_batches) +
                    " batches. ")
        for batch_idx, batch in enumerate(batches):
            n_points = len(batch)
            if describe:
                start = datetime.now()
            payload = {
                "units": units,
                "nPoints": n_points,
            }
            for idx, point in enumerate(batch):
                timekey = "points[" + str(idx) + "].time"
                payload[timekey] = point.time
                valuekey = "points[" + str(idx) + "].value"
                payload[valuekey] = point.value
            r = self.session.post(reqUrl, data=payload,
                verify=self.isRequestVerificationEnabled)
            if r.status_code != 202:
                print(r.status_code)
                print(r.text)
                raise ValueError
            if describe:
                end = datetime.now()
                timeElapsed = end - start
                id_ss = str(batch_idx+1)
                n_batches_ss = str(len_batches)
                elapsed_seconds = timeElapsed.total_seconds()
                n_seconds_ss = "{:.2f}".format(elapsed_seconds)
                pps_ss = "{:.2f}".format(n_points*1.0/1000.0/elapsed_seconds)
                if batch_idx + 1 != len_batches:
                    print("(appendPoints) request " + id_ss + "/" + n_batches_ss +
                        ". " + n_seconds_ss + " seconds, (" + pps_ss +
                        "*10^3 points per second)", end='\r')
                else:
                    print("\ninsert points fcall done")


