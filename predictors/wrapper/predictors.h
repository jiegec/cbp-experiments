#ifndef PREDICTOR
#define PREDICTOR(...)
#endif

// CBP2016
// https://jilp.org/cbp2016/program.html
// https://web.archive.org/web/20220814115014/http://hpca23.cse.tamu.edu/cbp2016/cbp2016.final.tar.gz
// AndreSeznecLimited/cbp8KB
PREDICTOR(AndreSeznec-TAGE-SC-L-8KB, AndreSeznecTAGESCL8KB)
// AndreSeznecLimited/cbp8KB without SC-L
PREDICTOR(AndreSeznec-TAGE-SC-L-8KB-Only-TAGE, AndreSeznecTAGESCL8KBOnlyTAGE)
// AndreSeznecLimited/cbp64KB
PREDICTOR(AndreSeznec-TAGE-SC-L-64KB, AndreSeznecTAGESCL64KB)
// AndreSeznecLimited/cbp64KB without SC-L
PREDICTOR(AndreSeznec-TAGE-SC-L-64KB-Only-TAGE, AndreSeznecTAGESCL64KBOnlyTAGE)
// AndreSeznecUnlimited/cbpUnlimited
PREDICTOR(AndreSeznec-Unlimited, AndreSeznecUnlimited)

// TAGE-SC, an engineering cookbook
// https://team.inria.fr/pacap/members/andre-seznec/
// https://files.inria.fr/pacap/seznec/TageCookBook/predictor.h
PREDICTOR(AndreSeznec-TAGE-Cookbook, AndreSeznecTAGECookbook)

// CBP2025, TAGE-SC 192KB without loop predictor
// https://ericrotenberg.wordpress.ncsu.edu/cbp2025/
// https://drive.google.com/file/d/14EJlnzk_avmiaYMNSRUGpPf7DLCAdJBq/view?usp=sharing
PREDICTOR(AndreSeznec-TAGE-SC-192KB, AndreSeznecTAGESC192KB)
