import sys
from bs4 import BeautifulSoup

def CardString(strCardRaw):
    strCardRaw = strCardRaw.replace("X", "Z")
    assert(len(strCardRaw)==2)
    assert(strCardRaw[0] in "EGHS")
    assert(strCardRaw[1] in "789ZUOKA")
    return strCardRaw

def OpenFileParseGame(strFile):
    with open(strFile) as fileHtml:
        soup = BeautifulSoup(fileHtml.read(), "html.parser")
        divplayers = soup.find(class_="players")
        vecdivplayer = divplayers.find_all("div")
        assert(4==len(vecdivplayer))
        dictstreplayerindex = {}
        strDataUsername = "data-username"
        for eplayerindex, divplayer in enumerate(vecdivplayer):
            a = divplayer.find("a")
            dictstreplayerindex[a[strDataUsername]] = eplayerindex
        for eplayerindex, divHand in enumerate(soup.find_all(class_="show-hand")):
            vecspancard = divHand.find_all("span")
            assert(len(vecspancard)==8)
            print(eplayerindex, end=" ")
            for spancard in vecspancard:
                print(CardString(spancard["class"][3]), end=" ")
            print("")
        for divtrickcontainer in soup.find_all(class_="content_full trick-container"):
            for divtricks in divtrickcontainer.find_all(class_="tricks"):
                vecdivcard = divtricks.find_all("div")
                assert(len(vecdivcard)==4)
                for divcard in vecdivcard:
                    vecstrClass = divcard["class"]
                    assert(len(vecstrClass)==3 and vecstrClass[-1] in ["highlighted", ""])
                    assert(vecstrClass[0]=="card")
                    assert(vecstrClass[1].startswith("position"))
                print(dictstreplayerindex[divtricks.find_all("div")[0].find("a")[strDataUsername]], end=" ")
                for divcard in vecdivcard:
                    spancard = divcard.find("span")
                    print(CardString(spancard["class"][-1]), end=" ")
                print("")
        dictnnPayout = {}
        for eplayerindex, divplayer in enumerate(vecdivplayer):
            strPayoutRaw = divplayer.find("p").find("span").contents[0]
            assert(strPayoutRaw.startswith("€ "))
            print(int(strPayoutRaw[2:].replace(",", "")), end=" ")

vecstrFiles = sys.argv[1:]

for strFile in vecstrFiles:
    print(strFile)
    OpenFileParseGame(strFile)


