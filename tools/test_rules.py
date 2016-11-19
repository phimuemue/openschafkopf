import sys
import os
from bs4 import BeautifulSoup

def CardString(strCardRaw):
    strCardRaw = strCardRaw.lower()
    strCardRaw = strCardRaw.replace("x", "z")
    assert(len(strCardRaw)==2)
    assert(strCardRaw[0] in "eghs")
    assert(strCardRaw[1] in "789zuoka")
    return strCardRaw

vecpairstrdictstrfnGame = [
    ("rufspiel", {
        "Sauspiel auf die Alte" : lambda eplayerindex: "SRulesRufspiel{m_eplayerindex: %d, m_efarbe: EFarbe::Eichel}"%(eplayerindex),
        "Sauspiel auf die Blaue" : lambda eplayerindex: "SRulesRufspiel{m_eplayerindex: %d, m_efarbe: EFarbe::Gras}"%(eplayerindex),
        "Sauspiel auf die Hundsgfickte" : lambda eplayerindex: "SRulesRufspiel{m_eplayerindex: %d, m_efarbe: EFarbe::Schelln}"%(eplayerindex),
    }),
    ("farbwenz", {
        "Eichel-Farbwenz" : lambda eplayerindex: "SRulesSoloLike::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Eichel-Wenz\")"%(eplayerindex),
        "Gras-Farbwenz" : lambda eplayerindex: "SRulesSoloLike::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Gras-Wenz\")"%(eplayerindex),
        "Herz-Farbwenz" : lambda eplayerindex: "SRulesSoloLike::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Herz-Wenz\")"%(eplayerindex),
        "Schelln-Farbwenz" : lambda eplayerindex: "SRulesSoloLike::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Schelln-Wenz\")"%(eplayerindex),
    }),
    ("wenz", {
        "Wenz" : lambda eplayerindex: "SRulesSoloLike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Wenz\")"%(eplayerindex),
    }),
    ("solo", {
        "Eichel-Solo" : lambda eplayerindex: "SRulesSoloLike::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Eichel-Solo\")"%(eplayerindex),
        "Gras-Solo" : lambda eplayerindex: "SRulesSoloLike::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Gras-Solo\")"%(eplayerindex),
        "Herz-Solo" : lambda eplayerindex: "SRulesSoloLike::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Herz-Solo\")"%(eplayerindex),
        "Schelln-Solo" : lambda eplayerindex: "SRulesSoloLike::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Schelln-Solo\")"%(eplayerindex),
    }),
    ("geier", {
        "Geier" : lambda eplayerindex: "SRulesSoloLike::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>::new(%d, VGameAnnouncementPriority::SoloLikeSimple(0), \"Geier\")"%(eplayerindex),
    }),
    ("ramsch", {
        "Ramscch" : lambda eplayerindex: "SRulesRamsch"%(eplayerindex),
    }),
]

def OpenFileParseGame(strFile, dictstrfnGame):
    strResult = ""
    def AppendToResultNoNewline(str):
        nonlocal strResult
        strResult = strResult + str
    def AppendToResult(str):
        nonlocal strResult
        strResult = strResult + str + "\n"
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
            return None
        AppendToResult("test_rules(")
        AppendToResult("    \"%s\","%(strFile))
        for strGame in dictstrfnGame:
            if h1Game.startswith(strGame):
                assert(not bGameFound)
                bGameFound = True
                AppendToResult("    &%s,"%(dictstrfnGame[strGame](dictstreplayerindex[h1Game.rsplit(" ")[-1]])))
        AppendToResultNoNewline("    [")
        hands = soup.find_all(class_="show-hand")
        if len(hands)==0:
            return "// %s has wrong format"%strFile
        assert(len(hands)==4)
        for eplayerindex, divHand in enumerate(hands):
            vecspancard = divHand.find_all("span")
            if len(vecspancard)!=8:
                if len(vecspancard)==6:
                    return "// %s // TODO kurze Karte"%strFile
                if len(vecspancard)==0:
                    return "// %s has wrong format"%strFile
                else:
                    assert(False)
                AppendToResult("Error in file: len(vecspancard)!=8")
                return strResult
            AppendToResultNoNewline("\"%s\"," % (" ".join([CardString(spancard["class"][3]) for spancard in vecspancard])))
        AppendToResult("],")
        # doubling
        AppendToResultNoNewline("    vec![")
        for aKlopfer in soup.find(text="Klopfer").find_parent("tr").find_all("a", href=True):
            AppendToResultNoNewline("%s,"%dictstreplayerindex[aKlopfer.contents[0]])
        AppendToResult("],")
        # Stoss (Kontra etc)
        AppendToResultNoNewline("    vec![")
        for aKontraRetour in soup.find(text="Kontra und Retour").find_parent("tr").find_all("a", href=True):
            AppendToResultNoNewline("%s,"%dictstreplayerindex[aKontraRetour.contents[0]])
        AppendToResult("],")
        AppendToResultNoNewline("    [")
        for divtrickcontainer in soup.find_all(class_="content_full trick-container"):
            for divtricks in divtrickcontainer.find_all(class_="tricks"):
                vecdivcard = divtricks.find_all("div")
                assert(len(vecdivcard)==4)
                for divcard in vecdivcard:
                    vecstrClass = divcard["class"]
                    assert(len(vecstrClass)==3 and vecstrClass[-1] in ["highlighted", ""])
                    assert(vecstrClass[0]=="card")
                    assert(vecstrClass[1].startswith("position"))
                AppendToResultNoNewline("(%d, \"%s\")," %(
                    dictstreplayerindex[divtricks.find_all("div")[0].find("a")[strDataUsername]],
                    " ".join(CardString(divcard.find_all("span")[-1]["class"][-1]) for divcard in vecdivcard)
                ))
        AppendToResult("],")
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
        AppendToResult("    [%s],"%", ".join([str(nPayout) for nPayout in anPayout]))
        AppendToResult(");")
    return strResult

strDir = sys.argv[1]

vecstrFile = []
for (strRoot, vecStrDir, vecstrFileRaw) in os.walk(strDir):
    for (strFile) in vecstrFileRaw:
        vecstrFile.append(strRoot + "/" + strFile)
vecstrFile.sort()

for (strGame, dictstrfnGame) in vecpairstrdictstrfnGame:
    print("\n#[test]")
    print("fn test_rules%s() {"%strGame)
    for strFile in vecstrFile:
        ostr = OpenFileParseGame(strFile, dictstrfnGame)
        if ostr:
            for strLine in ostr.splitlines():
                print("    "+strLine)
    print("}")

