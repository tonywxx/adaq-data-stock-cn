use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_CHINAMONEY: &str = "chinamoney";
const SOURCE_THS: &str = "10jqka";

// ---------------------------------------------------------------------------
// bond_buy_back_hist_em — 东方财富-质押式回购-历史数据
// https://quote.eastmoney.com/center/gridlist.html#bond_sh_buyback
// ---------------------------------------------------------------------------

/// 质押式回购历史行情行 (`bond_buy_back_hist_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondBuyBackHistRow {
    pub date: String,
    pub open: Option<f64>,
    pub close: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<f64>,
    pub amount: Option<f64>,
    pub source: &'static str,
}

/// 质押式回购历史数据 from Eastmoney (`bond_buy_back_hist_em`).
///
/// `symbol` is a 质押式回购代码 (e.g. `204001`). The market id is `0` when the
/// code starts with `1`, otherwise `1` (ported from akshare).
pub async fn bond_buy_back_hist_em(client: &Client, symbol: &str) -> Result<Vec<BondBuyBackHistRow>> {
    let market_id = if symbol.starts_with('1') { "0" } else { "1" };
    let secid = format!("{market_id}.{symbol}");
    let params = [
        ("secid", secid.as_str()),
        ("klt", "101"),
        ("fqt", "1"),
        ("lmt", "10000"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "bond_buy_back_hist_em",
            "https://push2his.eastmoney.com/api/qt/stock/kline/get",
            &params,
        )
        .await?;
    parse_bond_buy_back_hist(&v)
}

pub(crate) fn parse_bond_buy_back_hist(resp: &Value) -> Result<Vec<BondBuyBackHistRow>> {
    let klines = resp
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })?;
    let mut out = Vec::with_capacity(klines.len());
    for k in klines {
        let s = k.as_str().ok_or_else(|| Error::Parse {
            endpoint: "bond_buy_back_hist_em",
            message: "kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        out.push(BondBuyBackHistRow {
            date: p.first().map(|x| x.to_string()).unwrap_or_default(),
            open: p.get(1).and_then(|x| x.parse::<f64>().ok()),
            close: p.get(2).and_then(|x| x.parse::<f64>().ok()),
            high: p.get(3).and_then(|x| x.parse::<f64>().ok()),
            low: p.get(4).and_then(|x| x.parse::<f64>().ok()),
            volume: p.get(5).and_then(|x| x.parse::<f64>().ok()),
            amount: p.get(6).and_then(|x| x.parse::<f64>().ok()),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// bond_gb_us_sina — 新浪财经-美国国债收益率行情
// https://stock.finance.sina.com.cn/forex/globalbd/cn10yt.html
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// bond_zh_cov_info_ths — 同花顺-可转债行情
// https://data.10jqka.com.cn/ipo/bond/
// ---------------------------------------------------------------------------

/// 可转债行情行 (`bond_zh_cov_info_ths`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondZhCovInfoThsRow {
    pub bond_code: String,
    pub bond_name: String,
    pub sub_date: String,
    pub sub_code: String,
    pub share_code: String,
    pub stock_code: String,
    pub stock_name: String,
    pub sign_date: String,
    pub plan_total: Option<f64>,
    pub issue_total: Option<f64>,
    pub success_rate: Option<f64>,
    pub listing_date: String,
    pub expire_date: String,
    pub price: Option<f64>,
    pub quota: Option<f64>,
    pub source: &'static str,
}

/// 可转债行情 from 10jqka (`bond_zh_cov_info_ths`).
pub async fn bond_zh_cov_info_ths(client: &Client) -> Result<Vec<BondZhCovInfoThsRow>> {
    let v = client
        .get_json(
            SOURCE_THS,
            "bond_zh_cov_info_ths",
            "https://data.10jqka.com.cn/ipo/kzz/",
            &[],
        )
        .await?;
    parse_bond_zh_cov_info_ths(&v)
}

pub(crate) fn parse_bond_zh_cov_info_ths(resp: &Value) -> Result<Vec<BondZhCovInfoThsRow>> {
    let list = resp
        .get("list")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_THS,
            message: "missing list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(BondZhCovInfoThsRow {
            bond_code: opt_str_or(item, "bond_code", ""),
            bond_name: opt_str_or(item, "bond_name", ""),
            sub_date: opt_str_or(item, "sub_date", ""),
            sub_code: opt_str_or(item, "sub_code", ""),
            share_code: opt_str_or(item, "share_code", ""),
            stock_code: opt_str_or(item, "code", ""),
            stock_name: opt_str_or(item, "name", ""),
            sign_date: opt_str_or(item, "sign_date", ""),
            plan_total: opt_f64(item, "plan_total"),
            issue_total: opt_f64(item, "issue_total"),
            success_rate: opt_f64(item, "success_rate"),
            listing_date: opt_str_or(item, "listing_date", ""),
            expire_date: opt_str_or(item, "expire_date", ""),
            price: opt_f64(item, "price"),
            quota: opt_f64(item, "quota"),
            source: SOURCE_THS,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// bond_info_cm_query — 中国外汇交易中心-查询指标参数
// https://www.chinamoney.com.cn/chinese/scsjzqxx/
// ---------------------------------------------------------------------------

/// 查询指标参数行 (`bond_info_cm_query`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondInfoCmQueryRow {
    pub name: String,
    pub code: String,
    pub source: &'static str,
}

const CM_QUERY_SYMBOL_MAP: &[(&str, &str)] = &[
    ("债券类型", "bondType"),
    ("息票类型", "couponType"),
    ("发行年份", "issueYear"),
    ("评级等级", "bondRtngShrt"),
];

/// 查询指标参数 from ChinaMoney (`bond_info_cm_query`).
///
/// `symbol` is one of `主承销商`, `债券类型`, `息票类型`, `发行年份`, `评级等级`.
pub async fn bond_info_cm_query(client: &Client, symbol: &str) -> Result<Vec<BondInfoCmQueryRow>> {
    if symbol == "主承销商" {
        let v = client
            .post_form_json(
                SOURCE_CHINAMONEY,
                "bond_info_cm_query",
                "https://www.chinamoney.com.cn/ags/ms/cm-u-bond-md/EntyFullNameSearchCondition",
                &[],
                None,
            )
            .await?;
        let enty = v
            .get("data")
            .and_then(|d| d.get("enty"))
            .and_then(|e| e.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_CHINAMONEY,
                message: "missing data.enty".into(),
            })?;
        return Ok(enty
            .iter()
            .map(|item| BondInfoCmQueryRow {
                name: opt_str_or(item, "name", ""),
                code: opt_str_or(item, "code", ""),
                source: SOURCE_CHINAMONEY,
            })
            .collect());
    }

    let key = map_lookup(CM_QUERY_SYMBOL_MAP, symbol, "symbol")?;
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "bond_info_cm_query",
            "https://www.chinamoney.com.cn/ags/ms/cm-u-bond-md/BondBaseInfoSearchCondition",
            &[],
            None,
        )
        .await?;
    let arr = v
        .get("data")
        .and_then(|d| d.get(key.as_str()))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: format!("missing data.{key}"),
        })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Some(s) = item.as_str() {
            out.push(BondInfoCmQueryRow {
                name: s.to_string(),
                code: s.to_string(),
                source: SOURCE_CHINAMONEY,
            });
        } else {
            out.push(BondInfoCmQueryRow {
                name: opt_str_or(item, "name", ""),
                code: opt_str_or(item, "code", ""),
                source: SOURCE_CHINAMONEY,
            });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// bond_info_cm — 中国外汇交易中心-债券信息查询
// https://www.chinamoney.com.cn/chinese/scsjzqxx/
// ---------------------------------------------------------------------------

/// 债券信息查询行 (`bond_info_cm`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondInfoCmRow {
    pub bond_short_name: String,
    pub bond_code: String,
    pub issuer: String,
    pub bond_type: String,
    pub issue_date: String,
    pub latest_rating: String,
    pub query_code: String,
    pub source: &'static str,
}

/// 债券信息查询 from ChinaMoney (`bond_info_cm`).
///
/// All params mirror akshare (empty strings mean "no filter"). Chinese label
/// params (`bond_type`/`coupon_type`/`underwriter`) are resolved to their
/// ChinaMoney codes via [`bond_info_cm_query`].
#[allow(clippy::too_many_arguments)]
pub async fn bond_info_cm(
    client: &Client,
    bond_name: &str,
    bond_code: &str,
    bond_issue: &str,
    bond_type: &str,
    coupon_type: &str,
    issue_year: &str,
    underwriter: &str,
    grade: &str,
) -> Result<Vec<BondInfoCmRow>> {
    let bond_type_code = if !bond_type.is_empty() {
        resolve_cm_code(client, "债券类型", bond_type).await?
    } else {
        String::new()
    };
    let coupon_type_code = if !coupon_type.is_empty() {
        resolve_cm_code(client, "息票类型", coupon_type).await?
    } else {
        String::new()
    };
    let underwriter_code = if !underwriter.is_empty() {
        resolve_cm_code(client, "主承销商", underwriter).await?
    } else {
        String::new()
    };

    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params = [
            ("pageNo", page_s.as_str()),
            ("pageSize", "15"),
            ("bondName", bond_name),
            ("bondCode", bond_code),
            ("issueEnty", bond_issue),
            ("bondType", bond_type_code.as_str()),
            ("bondSpclPrjctVrty", ""),
            ("couponType", coupon_type_code.as_str()),
            ("issueYear", issue_year),
            ("entyDefinedCode", underwriter_code.as_str()),
            ("rtngShrt", grade),
        ];
        let v = client
            .post_form_json(
                SOURCE_CHINAMONEY,
                "bond_info_cm",
                "https://www.chinamoney.com.cn/ags/ms/cm-u-bond-md/BondMarketInfoList2",
                &params,
                None,
            )
            .await?;
        let data = v.get("data").ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing data".into(),
        })?;
        let total_page = data
            .get("pageTotal")
            .and_then(|t| t.as_u64())
            .unwrap_or(1);
        let result_list = data
            .get("resultList")
            .and_then(|r| r.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_CHINAMONEY,
                message: "missing data.resultList".into(),
            })?;
        for item in result_list {
            out.push(BondInfoCmRow {
                bond_short_name: opt_str_or(item, "bondName", ""),
                bond_code: opt_str_or(item, "bondCode", ""),
                issuer: opt_str_or(item, "entyFullName", ""),
                bond_type: opt_str_or(item, "bondType", ""),
                issue_date: opt_str_or(item, "issueStartDate", ""),
                latest_rating: opt_str_or(item, "debtRtng", ""),
                query_code: opt_str_or(item, "bondDefinedCode", ""),
                source: SOURCE_CHINAMONEY,
            });
        }
        if page as u64 >= total_page {
            break;
        }
        page += 1;
    }
    Ok(out)
}

async fn resolve_cm_code(client: &Client, query_symbol: &str, label: &str) -> Result<String> {
    let rows = bond_info_cm_query(client, query_symbol).await?;
    rows.into_iter()
        .find(|r| r.name == label)
        .map(|r| r.code)
        .ok_or_else(|| Error::InvalidParam(format!("unknown {query_symbol}: {label}")))
}

// ---------------------------------------------------------------------------
// bond_info_detail_cm — 中国外汇交易中心-债券详情
// https://www.chinamoney.com.cn/chinese/zqjc/
// ---------------------------------------------------------------------------

/// 债券详情行 (`bond_info_detail_cm`): a key/value pair from `bondBaseInfo`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondInfoDetailCmRow {
    pub name: String,
    pub value: String,
    pub source: &'static str,
}

/// 债券详情 from ChinaMoney (`bond_info_detail_cm`).
///
/// `symbol` is a 债券简称; resolved to its `bondDefinedCode` via [`bond_info_cm`].
pub async fn bond_info_detail_cm(client: &Client, symbol: &str) -> Result<Vec<BondInfoDetailCmRow>> {
    let info = bond_info_cm(client, symbol, "", "", "", "", "", "", "").await?;
    let query_code = info
        .first()
        .map(|r| r.query_code.clone())
        .ok_or_else(|| Error::InvalidParam(format!("no bond found for: {symbol}")))?;
    let params = [("bondDefinedCode", query_code.as_str())];
    let v = client
        .post_form_json(
            SOURCE_CHINAMONEY,
            "bond_info_detail_cm",
            "https://www.chinamoney.com.cn/ags/ms/cm-u-bond-md/BondDetailInfo",
            &params,
            None,
        )
        .await?;
    let data = v.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_CHINAMONEY,
        message: "missing data".into(),
    })?;
    let base = data
        .get("bondBaseInfo")
        .and_then(|b| b.as_object())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_CHINAMONEY,
            message: "missing data.bondBaseInfo".into(),
        })?;
    let mut out = Vec::with_capacity(base.len());
    for (k, val) in base {
        if k == "creditRateEntyList" || k == "exerciseInfoList" {
            continue;
        }
        let value = match val {
            Value::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        };
        out.push(BondInfoDetailCmRow {
            name: k.clone(),
            value,
            source: SOURCE_CHINAMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// bond_available_index_cbond — 中国债券信息网-可选项中债指数名称列表
// https://yield.chinabond.com.cn/cbweb-mn/indices/singleIndexQueryResult
//
// akshare returns a static list of index-category names from INDEX_MAPPING
// (no network call); we embed the same names as a local constant.
// ---------------------------------------------------------------------------

/// 中债指数名称列表行 (`bond_available_index_cbond`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BondAvailableIndexCbondRow {
    pub index: u32,
    pub value: String,
}

const INDEX_NAMES: &str = "\
新综合指数
高等级科技创新债券综合指数
长江养老年金基金债券指数
中信证券挂钩DR浮动利率政策性银行债活跃券指数
股份制商业银行同业存单指数
金融高质量发展主题信用债指数
交易所国债指数
进出口行债券总指数
市场隐含评级AA信用债指数
房地产行业信用债指数
重庆市地方政府债指数
甘肃省地方政府债指数
公司信用类科技创新债券指数
长三角绿色债券指数
企业债AA-指数
中国高等级债券指数
中高等级公司信用类债券指数
系统重要性银行债券指数
红利现金流高等级股债金波动率控制1.5%指数
中高等级粤港澳大湾区绿色债券指数
投资级公司信用债综合指数
中高等级绿色金融债券指数
高等级科技创新债券指数
北京农商银行中高信用等级农村商业银行金融债券指数
货币市场基金可投资债券指数
高信用等级债券指数
非银金融行业信用债指数
商业银行无固定期限资本债券市场隐含评级AA指数
京津冀公司信用类债券指数
京津冀债券综合指数
工行关键期限国债指数
广东省地方政府债指数
厦门市地方政府债指数
商业银行无固定期限资本债券AAA指数
粤港澳大湾区绿色债券综合指数
中高等级科技创新绿色普惠主题信用债指数
红利自由现金流低波股债恒定比例10/90指数
高信用等级企业债指数
农行乡村振兴债券指数
城市商业银行及农村商业银行同业存单AAA指数
股债恒定组合10/90指数
国信证券深圳市国有企业信用债精选指数
商业银行无固定期限资本债券指数
银行间碳中和债券指数
中高等级信用债综合指数
贵阳银行西部高质量发展信用债精选指数
投资优选国际信用评级投资级活跃信用债指数
红利现金流高等级股债金波动率控制1%指数
东方证券科技创新信用债精选指数
进出口行新发关键期限债券指数
商业银行二级资本债券AAA指数
粤港澳大湾区信用债指数
投资优选信用债指数
科技创新绿色普惠主题信用债指数
中国高等级债券指数(美元)
信用债总指数
高等级科技创新及绿色债券指数
中邮理财高等级绿色债券精选指数
平安人寿ESG整合策略信用债指数
战略性新兴产业信用债指数
商业银行债券AAA指数
AAA评级债券综合指数
高等级黄河流域绿色债券指数
高等级公司信用类债券综合指数
中信证券高等级同业存单指数
浦银理财新质生产力发展债券指数
银行间国债指数
交易所高等级科技创新债券指数
银行间资产支持证券指数
中国气候相关债券指数
招商银行优选信用债指数
福建省地方政府债指数
市场隐含评级AA+及以上信用债指数
青海省地方政府债指数
红利自由现金流低波股债恒定比例25/75指数
投资优选信用债分散指数
高信用等级城市商业银行及农村商业银行债券指数
金融机构二级资本债券总指数
新中期票据总指数
高收益中期票据指数
商业银行二级资本债券指数
高信用等级商业银行无固定期限及二级资本债券指数
银行间科技创新债券指数
金融行业信用债指数
高等级科技创新债券行业精选指数
固定利率债券指数
中信证券挂钩LPR浮动利率政策性银行债活跃券指数
广西壮族自治区公司信用类债券指数
长三角债券综合指数
个人住房抵押贷款资产支持证券指数
青岛市地方政府债指数
煤炭行业信用债指数
安徽省公司信用类债券指数
企业债AA+指数
挂钩DR浮动利率政策性银行债指数
红利自由现金流低波股债恒定比例20/80指数
金融机构科技创新债券指数
中豫信增河南省信用债指数
红利现金流中高等级股债金波动率控制1%指数
电力行业优质转型企业信用债指数
工行熊猫债30指数
申万宏源ESG绿色信用债精选指数
天府信用增进公司增信债券指数
京津冀绿色债券综合指数
高等级科技创新绿色普惠主题信用债指数
高信用等级数字经济产业信用债指数
中银理财高等级乡村振兴债券指数
国有大型商业银行及股份制商业银行债券指数
民营企业公司信用类债券指数
湖南省地方政府债指数
贵州省地方政府债指数
新疆维吾尔自治区（新疆生产建设兵团）地方政府债指数
国寿资产ESG信用债精选指数
中邮理财高等级科技创新债券精选指数
平安理财ESG优选绿色债券指数
中高等级公司债利差因子指数
长三角公司信用类债券指数
中信证券精选高等级资产支持证券指数
交易所科技创新债券指数
中银理财高信用等级同业存单指数
中银绿色债券指数
国有大型商业银行同业存单指数
中高等级长江经济带绿色债券指数
AAA信用债综合指数
红利自由现金流低波股债金恒定比例指数
申万宏源中小微企业主题优选信用债指数
企业债总指数
商业银行债券指数
公司债总指数
长江经济带债券综合指数
信用债价值因子权重调整策略指数
黑龙江省地方政府债指数
碳中和绿色债券指数
投资优选政策性金融债指数
中金公司乡村振兴信用债精选指数
高等级公司信用类债券指数
京津冀科技创新债券指数
渤海银行新质生产力主题科技创新债券指数
长三角地方政府债指数
AAA科技创新债券指数
投资级中资美元债指数
成渝地区双城经济圈国有企业信用债精选指数
投资优选活跃信用债指数
北银理财绿色发展风险平价指数
中高等级科技创新债券综合指数
个人住房抵押贷款资产支持证券精选指数
红利自由现金流低波股债金恒定比例10/85/5指数
高信用等级同业存单指数
中国绿色债券精选指数
长三角中高等级信用债指数
江西省地方政府债指数
江苏省地方政府债指数
中信证券国债及地方政府债精选指数
国债及政策性银行债指数
高信用等级公司信用类债券综合指数
银行间高等级科技创新债券指数
公路行业信用债指数
投资优选国际信用评级投资级信用债分散指数
高信用等级农村商业银行债券优选指数
科技创新债券指数
科技创新债券综合指数
高等级信用债指数
中高等级公司信用类债券综合指数
中高等级京津冀绿色债券指数
城市商业银行及农村商业银行债券AAA指数
浦发银行绿色低碳股债优选指数
金融债券总指数
离岸人民币中国主权及政策性金融债指数
安徽省地方政府债指数
AA国有企业信用债优选指数
中高等级信用债指数
工行熊猫债AAA指数
国有大型商业银行及股份制商业银行同业存单指数
黄金保值国开行债券风险平价指数
交行长三角ESG优选信用债指数
中信证券ESG优选信用债指数
数字经济产业信用债指数
投资级公司信用债精选指数
中信证券久期轮动政策性金融债指数
红利自由现金流低波股债恒定比例30/70指数
高等级信用债综合指数
投资级公司科技创新债券精选指数
交易所信用债AAA指数
高信用等级城市商业银行及农村商业银行同业存单指数
高信用等级商业银行债券指数
投资优选科技创新债券指数
固定利率金融债指数
银行间高等级碳中和债券指数
同业存单AA+指数
平安-可投资级信用债指数
农发行债券总指数
浙江省地方政府债指数
上海市地方政府债指数
陕西省地方政府债指数
资产支持证券指数
吉林省地方政府债指数
海南省地方政府债指数
ESG优选信用债指数
中金公司绿色资产支持证券指数
市场隐含评级AA+信用债指数
同业存单总指数
公司信用类债券指数
高收益企业债指数
企业债AAA指数
AAA公司信用类债券综合指数
AAA公司信用类债券指数
投资优选绿色债券指数
系统重要性银行同业存单指数
投资级主题绿色债券优选指数
高等级央企信用债精选指数
红利现金流中高等级股债波动率控制1.5%指数
中高等级科技创新及绿色债券指数
金融高质量发展主题债券综合指数
商业银行无固定期限及二级资本债券指数
国开行债券总指数
综合指数
固定利率企业债指数
中期票据总指数
黄金保值债券风险平价指数
广西壮族自治区地方政府债指数
新疆维吾尔自治区地方政府债指数
天津市地方政府债指数
投资优选综合指数
绿色债券综合指数
市场隐含评级AAA信用债指数
中国铁路债券指数
企业债AA指数
地方政府债指数
国泰海通陕川渝国企信用增强债券精选指数
银行间高等级绿色债券指数
红利现金流高等级股债波动率控制1.5%指数
红利自由现金流低波股债恒定比例5/95指数
绿色普惠主题金融债券优选指数
建筑工程行业信用债指数
长三角绿色债券综合指数
银行间债券总指数
交易所AAA科技创新债券指数
AAA信用债指数
银行间市场信用债AAA指数
共同富裕主题债券指数
中国绿色债券指数
优选投资级信用债指数
投资优选地方政府债指数
湖北省地方政府债指数
兴业绿色债券指数
银行普通债券AAA指数
中期票据AAA指数
黄金保值信用债风险平价指数
浦发银行ESG精选债券指数
北银理财京津冀企业高质量发展多元投资指数
固定利率国债指数
浮动利率金融债指数
AAA公司信用类科技创新债券指数
云南省地方政府债指数
辽宁省地方政府债指数
长江经济带绿色债券综合指数
关键期限国债指数
红利现金流中高等级股债波动率控制1%指数
北京银行高信用等级城市商业银行债券指数
中信证券个人汽车抵押贷款资产支持证券指数
中高等级长三角绿色债券指数
天风国际ESG优选中资美元债指数
工行绿色债券指数
红利现金流中高等级股债金波动率控制1.5%指数
京津冀绿色债券指数
河南省地方政府债指数
内蒙古自治区地方政府债指数
制造行业信用债指数
粤港澳大湾区绿色债券指数
科技创新及绿色债券指数
投资级公司绿色债券精选指数
市场隐含评级AAA+信用债指数
高等级公司信用类科技创新债券指数
银行间绿色债券指数
高等级战略性新兴产业信用债指数
红利自由现金流低波股债恒定比例15/85指数
电力行业信用债指数
科技创新绿色普惠主题债券综合指数
中高等级战略性新兴产业信用债指数
浮动利率债券指数
京津冀地方政府债指数
宁波市地方政府债指数
钢铁行业信用债指数
北京市地方政府债指数
山西省地方政府债指数
宁夏回族自治区地方政府债指数
西藏自治区地方政府债指数
投资优选国债指数
挂钩LPR浮动利率政策性银行债指数
银行金融债券AAA指数
市场隐含评级AAA-信用债指数
国有大型商业银行及股份制商业银行二级资本债券指数
高信用等级中期票据指数
高信用等级银行金融债券指数
同业存单AAA指数
中高等级科技创新债券指数
商业银行二级资本债券市场隐含评级AA指数
高等级绿色公司信用类债券指数
银行间AAA科技创新债券指数
黄河流域绿色债券综合指数
国债总指数
长江经济带地方政府债指数
山东省地方政府债指数
四川省地方政府债指数
深圳市地方政府债指数
河北省地方政府债指数
大连市地方政府债指数
信用债价值因子精选策略指数
粤港澳大湾区债券综合指数
利差驱动股债稳健指数
中债信用增进公司增信债券指数
银行金融债券指数
乡村振兴债券综合指数";

/// 可选项中债指数名称列表 (`bond_available_index_cbond`). Static, no network.
pub async fn bond_available_index_cbond(_client: &Client) -> Result<Vec<BondAvailableIndexCbondRow>> {
    let rows = INDEX_NAMES
        .lines()
        .enumerate()
        .map(|(i, name)| BondAvailableIndexCbondRow {
            index: (i as u32) + 1,
            value: name.to_string(),
        })
        .collect();
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn map_lookup(map: &[(&str, &str)], key: &str, kind: &str) -> Result<String> {
    for &(k, v) in map {
        if k == key {
            return Ok(v.to_string());
        }
    }
    Err(Error::InvalidParam(format!("unknown {kind}: {key}")))
}

#[allow(dead_code)]
fn num_val(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"));
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_bond_buy_back_hist_em() {
        let v = fixture("bond_buy_back_hist_em");
        let rows = parse_bond_buy_back_hist(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(2.100));
        assert_eq!(rows[0].close, Some(2.120));
        assert_eq!(rows[0].volume, Some(1000.0));
        assert_eq!(rows[1].date, "2024-01-03");
    }

    #[test]
    fn parses_bond_zh_cov_info_ths() {
        let v = fixture("bond_zh_cov_info_ths");
        let rows = parse_bond_zh_cov_info_ths(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].bond_code, "113065");
        assert_eq!(rows[0].bond_name, "齐鲁转债");
        assert_eq!(rows[0].stock_code, "600918");
        assert_eq!(rows[0].issue_total, Some(80.0));
        assert_eq!(rows[1].bond_name, "上银转债");
    }

    #[test]
    fn parses_bond_info_cm_query() {
        let v = fixture("bond_info_cm_query");
        let arr = v
            .get("data")
            .unwrap()
            .get("bondRtngShrt")
            .unwrap()
            .as_array()
            .unwrap();
        // Mirror the else-branch parse (object items with code/name).
        let out: Vec<BondInfoCmQueryRow> = arr
            .iter()
            .map(|item| BondInfoCmQueryRow {
                name: opt_str_or(item, "name", ""),
                code: opt_str_or(item, "code", ""),
                source: SOURCE_CHINAMONEY,
            })
            .collect();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "A-1");
        assert_eq!(out[0].code, "A-1");
        assert_eq!(out[1].name, "AAA");
    }

    #[test]
    fn parses_bond_info_cm() {
        let v = fixture("bond_info_cm");
        let data = v.get("data").unwrap();
        let total_page = data.get("pageTotal").unwrap().as_u64().unwrap();
        assert_eq!(total_page, 1);
        let list = data.get("resultList").unwrap().as_array().unwrap();
        let out: Vec<BondInfoCmRow> = list
            .iter()
            .map(|item| BondInfoCmRow {
                bond_short_name: opt_str_or(item, "bondName", ""),
                bond_code: opt_str_or(item, "bondCode", ""),
                issuer: opt_str_or(item, "entyFullName", ""),
                bond_type: opt_str_or(item, "bondType", ""),
                issue_date: opt_str_or(item, "issueStartDate", ""),
                latest_rating: opt_str_or(item, "debtRtng", ""),
                query_code: opt_str_or(item, "bondDefinedCode", ""),
                source: SOURCE_CHINAMONEY,
            })
            .collect();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].bond_short_name, "19渝机电CP002");
        assert_eq!(out[0].bond_code, "041900001");
        assert_eq!(out[0].query_code, "egfjh08154");
        assert_eq!(out[1].latest_rating, "AAA");
    }

    #[test]
    fn parses_bond_info_detail_cm() {
        let v = fixture("bond_info_detail_cm");
        let base = v
            .get("data")
            .unwrap()
            .get("bondBaseInfo")
            .unwrap()
            .as_object()
            .unwrap();
        let mut out = Vec::new();
        for (k, val) in base {
            if k == "creditRateEntyList" || k == "exerciseInfoList" {
                continue;
            }
            let value = match val {
                Value::String(s) => s.clone(),
                other => serde_json::to_string(other).unwrap_or_default(),
            };
            out.push(BondInfoDetailCmRow {
                name: k.clone(),
                value,
                source: SOURCE_CHINAMONEY,
            });
        }
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|r| r.name == "bondName" && r.value == "19渝机电CP002"));
        assert!(out.iter().any(|r| r.name == "bondCode"));
    }

    #[test]
    fn bond_available_index_cbond_static() {
        let fixture_v = fixture("bond_available_index_cbond");
        let expected: Vec<String> = fixture_v
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let rows = rt
            .block_on(bond_available_index_cbond(&Client::new()))
            .unwrap();
        assert_eq!(rows.len(), expected.len());
        assert_eq!(rows.len(), 313);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].value, expected[0]);
        assert_eq!(rows[0].value, "新综合指数");
    }
}
