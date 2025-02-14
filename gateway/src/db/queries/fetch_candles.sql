SELECT 
    open::TEXT as o, 
    close::TEXT as c, 
    low::TEXT as l, 
    high::TEXT as h, 
    ts::TEXT as t
FROM 
    candles
WHERE 
    ts <= $1
    AND resolution = $2
    AND multipool_id = $3
ORDER BY 
    ts DESC
LIMIT $4;
