import sys
import glob
from bs4 import BeautifulSoup

def CardString(strCardRaw):
    strCardRaw = strCardRaw.lower()
    strCardRaw = strCardRaw.replace("x", "z")
    assert(len(strCardRaw)==2)
    assert(strCardRaw[0] in "eghs")
    assert(strCardRaw[1] in "789zuoka")
    return strCardRaw

dictstrfnGame = {
    #"Sauspiel auf die Alte" : lambda eplayerindex: "SRulesRufspiel{m_eplayerindex: %d, m_efarbe: EFarbe::Eichel}"%(eplayerindex),
    #"Sauspiel auf die Blaue" : lambda eplayerindex: "SRulesRufspiel{m_eplayerindex: %d, m_efarbe: EFarbe::Gras}"%(eplayerindex),
    #"Sauspiel auf die Hundsgfickte" : lambda eplayerindex: "SRulesRufspiel{m_eplayerindex: %d, m_efarbe: EFarbe::Schelln}"%(eplayerindex),

    #"Eichel-Solo" : lambda eplayerindex: "*generate_sololike!(%d, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, \"Eichel-Solo\")"%(eplayerindex),
    #"Gras-Solo" : lambda eplayerindex: "*generate_sololike!(%d, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, \"Gras-Solo\")"%(eplayerindex),
    #"Herz-Solo" : lambda eplayerindex: "*generate_sololike!(%d, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, \"Herz-Solo\")"%(eplayerindex),
    #"Schelln-Solo" : lambda eplayerindex: "*generate_sololike!(%d, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>, \"Schelln-Solo\")"%(eplayerindex),

    "Wenz" : lambda eplayerindex: "*generate_sololike!(%d, SCoreGenericWenz<STrumpfDeciderNoTrumpf>, \"Wenz\")"%(eplayerindex),
    #"Eichel-Wenz" : lambda eplayerindex: "*generate_sololike!(%d, SCoreFarbwenz<SFarbeDesignatorEichel>), \"Eichel-Wenz\""%(eplayerindex),
    #"Gras-Wenz" : lambda eplayerindex: "*generate_sololike!(%d, SCoreFarbwenz<SFarbeDesignatorGras>), \"Gras-Wenz\""%(eplayerindex),
    #"Herz-Wenz" : lambda eplayerindex: "*generate_sololike!(%d, SCoreFarbwenz<SFarbeDesignatorHerz>), \"Herz-Wenz\""%(eplayerindex),
    #"Schelln-Wenz" : lambda eplayerindex: "*generate_sololike!(%d, SCoreFarbwenz<SFarbeDesignatorSchelln>), \"Schelln-Wenz\""%(eplayerindex),

    #"Geier" : lambda eplayerindex: "SRulesGeier"%(eplayerindex),
    #"Eichel-Farbgeier" : lambda eplayerindex: "SRulesFarbgeier{EFarbe::Eichel, %d}"%(eplayerindex),
    #"Gras-Farbgeier" : lambda eplayerindex: "SRulesFarbgeier{EFarbe::Gras, %d}"%(eplayerindex),
    #"Herz-Farbgeier" : lambda eplayerindex: "SRulesFarbgeier{EFarbe::Herz, %d}"%(eplayerindex),
    #"Schelln-Farbgeier" : lambda eplayerindex: "SRulesFarbgeier{EFarbe::Schelln, %d}"%(eplayerindex),

    #"Ramscch" : lambda eplayerindex: "SRulesRamsch"%(eplayerindex),
}

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
        h1Game = soup.find(class_="game-name-title").contents[0]
        bGameFound = False
        for strGame in dictstrfnGame:
            if h1Game.startswith(strGame):
                break
        else:
            return
        print("test_rules(")
        print("    \"%s\","%(strFile))
        for strGame in dictstrfnGame:
            if h1Game.startswith(strGame):
                assert(not bGameFound)
                bGameFound = True
                print("    &%s,"%(dictstrfnGame[strGame](dictstreplayerindex[h1Game.rsplit(" ")[-1]])))
        print("    [", end="")
        for eplayerindex, divHand in enumerate(soup.find_all(class_="show-hand")):
            vecspancard = divHand.find_all("span")
            if len(vecspancard)!=8:
                print("Error in file: len(vecspancard)!=8")
                return
            print("\"%s\"," % (" ".join([CardString(spancard["class"][3]) for spancard in vecspancard])), end="")
        print("],")
        print("    [", end="")
        for divtrickcontainer in soup.find_all(class_="content_full trick-container"):
            for divtricks in divtrickcontainer.find_all(class_="tricks"):
                vecdivcard = divtricks.find_all("div")
                assert(len(vecdivcard)==4)
                for divcard in vecdivcard:
                    vecstrClass = divcard["class"]
                    assert(len(vecstrClass)==3 and vecstrClass[-1] in ["highlighted", ""])
                    assert(vecstrClass[0]=="card")
                    assert(vecstrClass[1].startswith("position"))
                print("(%d, \"%s\")," %(
                    dictstreplayerindex[divtricks.find_all("div")[0].find("a")[strDataUsername]],
                    " ".join(CardString(divcard.find_all("span")[-1]["class"][-1]) for divcard in vecdivcard)
                ), end="")
        print("],")
        dictnnPayout = {}
        anPayout = []
        for eplayerindex, divplayer in enumerate(vecdivplayer):
            strPayoutRaw = divplayer.find("p").find("span").contents[0]
            assert(strPayoutRaw.startswith("€ ") or strPayoutRaw.startswith("P ") or strPayoutRaw.startswith("TP"))
            if strPayoutRaw.startswith("TP"):
                anPayout.append(int(strPayoutRaw[3:].replace(",", "")))
            else:
                anPayout.append(int(strPayoutRaw[2:].replace(",", "")))

        assert(4==len(anPayout))
        nPayoutSum = sum(anPayout)
        if 0!=nPayoutSum:
            nPayoutSum = -nPayoutSum
            assert(nPayoutSum > 0)
            nWinningPlayers = sum([1 for nPayout in anPayout if nPayout>0])
            assert(0==nPayoutSum%nWinningPlayers)
            for eplayerindex in range(4):
                if anPayout[eplayerindex]>0:
                    anPayout[eplayerindex] = int(anPayout[eplayerindex] + nPayoutSum/nWinningPlayers)
        assert(4==len(anPayout))
        assert(0==sum(anPayout))
        print("    [%s],"%", ".join([str(nPayout) for nPayout in anPayout]))
        print(");")

strGlob = sys.argv[1]

#print(list(glob.glob(strGlob)))

for strFile in glob.glob(strGlob):
    OpenFileParseGame(strFile)

