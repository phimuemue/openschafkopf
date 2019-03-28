import sys
import os
from bs4 import BeautifulSoup

def CardString(strCardRaw):
    strCardRaw = strCardRaw.lower()
    strCardRaw = strCardRaw.replace("x", "z")
    assert(len(strCardRaw)==2)
    assert(strCardRaw[0] in "eghs")
    assert(strCardRaw[1] in "789zuoka")
    return strCardRaw.upper()

def solo_payout_schneider_schwarz(tarif):
    if not tarif:
        return 10
    if len(tarif)==3:
        return tarif[0]
    assert(len(tarif)==2)
    return tarif[0]

def solo_payout(tarif):
    if not tarif:
        return 50
    if len(tarif)==3:
        return tarif[2]
    assert(len(tarif)==2)
    return tarif[1]

vecpairstrdictstrfnGame = [
    ("rufspiel", {
        "Sauspiel auf die Alte" : lambda eplayerindex, tarif: "&rulesrufspiel_new_test(EPlayerIndex::EPI%d, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3))"%(eplayerindex),
        "Sauspiel auf die Blaue" : lambda eplayerindex, tarif: "&rulesrufspiel_new_test(EPlayerIndex::EPI%d, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3))"%(eplayerindex),
        "Sauspiel auf die Hundsgfickte" : lambda eplayerindex, tarif: "&rulesrufspiel_new_test(EPlayerIndex::EPI%d, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3))"%(eplayerindex),
    }),
    ("farbwenz", {
        "Eichel-Farbwenz" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Eichel, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
        "Gras-Farbwenz" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
        "Herz-Farbwenz" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
        "Schelln-Farbwenz" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Schelln, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
    }),
    ("wenz", {
        "Wenz" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 2))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
    }),
    ("solo", {
        "Eichel-Solo" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
        "Gras-Solo" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
        "Herz-Solo" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
        "Schelln-Solo" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, EFarbe::Schelln, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 3))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
    }),
    ("geier", {
        "Geier" : lambda eplayerindex, tarif: "sololike(EPlayerIndex::EPI%d, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/%d, /*n_payout_schneider_schwarz*/%d, SLaufendeParams::new(10, 2))).upcast()"%(eplayerindex, solo_payout(tarif), solo_payout_schneider_schwarz(tarif)),
    }),
    ("ramsch", {
        "Ramscch" : lambda eplayerindex, tarif: "SRulesRamsch"%(eplayerindex),
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
        tarif = soup.find(text="Tarif")
        if tarif:
            tarif = soup.find(text="Tarif").find_parent("tr").find("td").contents[0].strip().split(" ", 1)[1].replace(",","").split(" / ")
            tarif = [int(n) for n in tarif]
        for strGame in dictstrfnGame:
            if h1Game.startswith(strGame):
                assert(not bGameFound)
                bGameFound = True
                AppendToResult("    %s,"%(dictstrfnGame[strGame](dictstreplayerindex[h1Game.rsplit(" ")[-1]], tarif if tarif else [10, 20, 50])))
        AppendToResultNoNewline("    [")
        hands = soup.find_all(class_="show-hand")
        if len(hands)==0:
            return "// %s has wrong format"%strFile
        assert(len(hands)==4)
        for eplayerindex, divHand in enumerate(hands):
            vecspancard = divHand.find_all("span")
            if len(vecspancard)!=8 and len(vecspancard)!=6:
                if len(vecspancard)==0:
                    return "// %s has wrong format"%strFile
                else:
                    assert(False)
                AppendToResult("Error in file: len(vecspancard)!=8")
                return strResult
            AppendToResultNoNewline("[%s]," % (",".join([CardString(spancard["class"][3]) for spancard in vecspancard])))
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
        AppendToResultNoNewline("    &[")
        for divtrickcontainer in soup.find_all(class_="content_full trick-container"):
            for divtricks in divtrickcontainer.find_all(class_="tricks"):
                vecdivcard = divtricks.find_all("div")
                assert(len(vecdivcard)==4)
                for divcard in vecdivcard:
                    vecstrClass = divcard["class"]
                    assert(len(vecstrClass)==3 and vecstrClass[-1] in ["highlighted", ""])
                    assert(vecstrClass[0]=="card")
                    assert(vecstrClass[1].startswith("position"))
                AppendToResultNoNewline("(%d, [%s])," %(
                    dictstreplayerindex[divtricks.find_all("div")[0].find("a")[strDataUsername]],
                    ",".join(CardString(divcard.find_all("span")[-1]["class"][-1]) for divcard in vecdivcard)
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

